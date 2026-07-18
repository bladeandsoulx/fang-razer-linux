# DDC Discovery Backoff and Circuit Breaker

**Date:** 2026-07-18
**Status:** Draft — revision 2 ready for review
**Target:** Fang daemon, protocol, and External Monitor UI

## Problem

`fangd` currently starts `ddcutil` automatically during peripheral discovery
and then retries discovery at a fixed 15-second interval while no monitor is
cached. On the affected machine, the installed `ddcutil 1.4.1` process has
repeatedly terminated with `SIGSEGV`. The daemon survives, but every retry can
probe the display/GPU stack again, produce another crash, and delay daemon
startup.

The current DDC wrapper collapses crashes, timeouts, unsupported features, and
ordinary "no monitor" results into the same unavailable result. Its shared
process runner also captures output without a limit and stops applying its
deadline after the direct child exits, even if a descendant keeps an output
pipe open. Fang therefore cannot slow down normal absence independently from
protecting the machine against a failing or wedged helper.

## Goals

- Preserve automatic external-monitor discovery.
- Use bounded exponential backoff when no DDC monitor is connected.
- Detect helper signals, timeouts, truncated output, and runner failures as
  typed hard failures.
- Stop automatic DDC work after repeated hard failures.
- Allow guarded manual recovery immediately and one automatic recovery probe
  after a long cooldown.
- Prevent manual requests from bypassing hard-failure safety delays.
- Serialize every `ddcutil` invocation so scans and setting changes cannot
  race.
- Make process execution cancellable and guarantee child cleanup on timeout
  and daemon shutdown.
- Keep fan, thermal, socket, GPU, network, and EC behavior independent from DDC
  failures.
- Publish cache and health atomically and show why DDC is waiting or paused.
- Make all policy behavior testable without real monitor hardware.

## Non-goals

- Replacing or repairing `ddcutil`.
- Adding udev/DRM hotplug listeners.
- Moving DDC into a second systemd service.
- Persisting circuit-breaker state across daemon restarts.
- Changing internal-panel brightness or refresh-rate behavior.
- Changing GPU-mode switching behavior.
- Adding simultaneous control of multiple external monitors.

## Verified Runtime Constraints

The execution design is based on these verified constraints:

- The current DDC path runs synchronous code through `spawn_blocking`.
  [Tokio documents that a started blocking task cannot be aborted][tokio-blocking],
  so aborting its join handle is not a shutdown mechanism.
- Tokio child handles do not provide strict cleanup merely by being dropped.
  [Tokio recommends explicitly waiting or killing when cleanup guarantees
  matter][tokio-process].
- Rust's `Instant` uses an OS-specific clock and its implementation may change.
  [The current Unix implementation uses `CLOCK_MONOTONIC`][rust-instant].
- On Linux, [`CLOCK_MONOTONIC` explicitly excludes suspended time][linux-clock].

[tokio-blocking]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
[tokio-process]: https://docs.rs/tokio/latest/tokio/process/struct.Command.html
[rust-instant]: https://doc.rust-lang.org/stable/std/time/struct.Instant.html
[linux-clock]: https://man7.org/linux/man-pages/man2/clock_gettime.2.html

## Selected Approach

Use one daemon-local DDC supervisor backed by a pure
`DdcDiscoveryPolicy` state machine and an asynchronous bounded process runner.
Automatic timers, manual rescans, brightness changes, and color changes all
pass through the supervisor. The supervisor owns the live DDC cache and is the
only component allowed to start `ddcutil`.

The supervisor does not await an opaque blocking task. It selects between
commands, deadlines, resume notifications, shutdown, and an explicit active
operation future. This provides one safety boundary while leaving the thermal
loop, socket server, GPU worker, and EC control independent.

## Async Process Runner Contract

### Execution model

DDC uses `tokio::process::Command`, with stdin disconnected and stdout/stderr
piped. On Unix, every helper starts in a new process group with the child's PID
as the process-group ID. `kill_on_drop` may be enabled as a last-resort safety
net, but it is not the cleanup contract.

The runner concurrently polls:

- direct-child exit;
- stdout drain;
- stderr drain;
- the operation deadline;
- an explicit shutdown-cancellation signal.

The deadline begins immediately before spawn and remains active until all three
normal-completion conditions are true: the direct child exited and was reaped,
stdout reached EOF, and stderr reached EOF. A direct child that exits while a
background descendant retains either pipe therefore still times out.

### Exact output bounds

`PROCESS_CAPTURE_LIMIT` is 64 KiB per stream. Each drain uses a fixed-size read
buffer:

1. retain bytes until the stream's 64 KiB limit is reached;
2. set that stream's `truncated` flag upon the first excess byte;
3. continue draining to EOF while discarding all excess bytes.

The returned process result contains separate `stdout_truncated` and
`stderr_truncated` flags. It never allocates in proportion to helper output.
For any `ddcutil` operation, either truncation flag is a hard
`output_truncated` failure, regardless of exit status, because parsing a
partial helper response is unsafe and an unexpected output flood is abnormal.

Only the first 1 KiB of a captured diagnostic may be flattened to one line for
the journal. Process output is never included in socket protocol state.

### Cleanup invariant

Every path after a successful spawn does exactly one of the following:

- awaits normal exit, reaps the direct child, and drains both pipes to EOF; or
- sends `SIGKILL` to the complete Unix process group, also requests direct-child
  kill as a fallback, then awaits/reaps the direct child and drains both pipes
  to EOF before returning.

Timeout and shutdown cancellation take the second path. A pipe-read failure
also kills and reaps the process group, closes the failed pipe, and drains the
other pipe to EOF. Transient wait errors are retried; an unrecoverable wait
error kills the group and keeps waiting until the child is reaped or the OS
reports that it was already reaped. The active operation future is never simply
dropped, including when its requesting client disconnects. No later DDC
operation starts until cleanup completes.

The supported cleanup bound after `SIGKILL` is two seconds. Exceeding it is a
hardware-acceptance failure and is logged as stuck cleanup; the supervisor
remains cleanup-locked and still refuses to abandon or overlap the child. This
exception may outlive a client request because no finite response deadline can
both guarantee reaping and handle a kernel task that cannot yet be reaped.

The existing synchronous GPU helper path remains behaviorally unchanged, but
its shared capture implementation receives the same 64 KiB limits, full
exit-plus-EOF deadline, and kill/reap invariant so `process.rs` has no unbounded
fallback.

### Process result types

The runner preserves:

- spawn failure, including `NotFound`;
- normal exit code;
- signal termination on Unix;
- timeout;
- stdout/stderr read failure;
- wait/reap failure;
- output-truncated flags;
- shutdown cancellation.

Shutdown cancellation is an orderly daemon lifecycle outcome and never
contributes to DDC policy counters. Existing non-DDC callers may map the typed
result back to their existing user-facing messages.

## DDC Command Runner and Discovery

DDC calls a small injectable asynchronous command-runner interface. Production
uses the bounded runner. Tests use a scripted fake which can pause, observe
cancellation, and return success, nonzero exit, timeout, missing executable,
truncated output, runner failure, or signal termination without touching
hardware.

Every DDC operation has a diagnostic label:

- module preparation;
- display detection;
- capabilities;
- color read;
- brightness read;
- color write;
- brightness write.

`modprobe i2c-dev` remains best-effort. It has a three-second deadline. A normal
nonzero result is not a hard DDC failure because the module may already be
built in or loaded. A timeout, signal, truncated output, or internal runner
failure during module preparation is logged, but the discovery sequence still
continues to `ddcutil detect`; only `ddcutil` outcomes drive the DDC breaker.

Every `ddcutil` invocation has an eight-second exit-plus-EOF deadline. A full
discovery has a maximum pre-cleanup execution-deadline sum of 35 seconds: three
seconds for `modprobe` and four eight-second DDC stages. Hard failure aborts the
sequence, so a timed-out DDC stage does not consume later stage budgets.

### Monitor definition and selection

A DDC monitor is present when `ddcutil detect` completes normally and its
output contains a valid numeric `Display N` entry. Entries explicitly reported
as invalid, including laptop eDP panels, do not qualify.

For this release, Fang selects the first valid `Display N` in `ddcutil` output,
preserving current behavior. Other valid monitors are ignored until a later
manual rediscovery changes the selected entry. Multi-monitor selection is a
separate feature.

The selected monitor becomes `ready` even if both optional controls are absent:

- VCP `0x10` determines brightness availability;
- VCP `0x14` determines color-preset availability.

A monitor exposing neither feature still sets `monitor_present = true`, stops
automatic discovery, and appears as detected with no supported controls.
`color_ddc` retains its legacy, narrower meaning: usable VCP `0x14` color
presets are present.

### Discovery sequence

A scan performs:

1. best-effort `modprobe i2c-dev`;
2. `ddcutil detect`;
3. capabilities for the selected display;
4. current color-preset read;
5. current brightness read.

The sequence checks shutdown cancellation before every stage and never starts a
later helper after cancellation has been observed.

No valid display at stage 2 is `NotFound`, regardless of a normal zero or
nonzero exit. Normal nonzero exits at optional stages 3–5 make only that
feature unavailable and still produce `Found`. A hard failure at stages 2–5
aborts all later commands.

## Typed DDC Outcomes

### `Found`

`Found` means a monitor was selected and discovery completed without a hard
helper failure. Optional features may be absent.

### `NotFound`

`NotFound` means display detection completed as a normal process exit but
contained no valid DDC display. It is expected when a monitor is unplugged and
never contributes to the breaker count.

### `SoftFailure`

A normal nonzero exit during a setting operation, such as an unsupported VCP,
monitor-busy response, or disabled DDC/CI setting, is a soft failure. It is
returned to the setting caller, invalidates the cached monitor, and schedules a
15-second rediscovery. Because the helper completed normally, it ends any
consecutive hard-failure streak.

Normal nonzero optional discovery stages are feature absence, not
`SoftFailure`.

### `HardFailure`

The following are hard:

- deadline expiration;
- termination by a signal, including `SIGSEGV`;
- stdout/stderr truncation;
- failure while reading, waiting for, or reaping a spawned helper;
- an equivalent internal runner failure after the child started;
- a spawn failure other than an absent executable.

A hard failure aborts the current DDC sequence immediately.

### `Unavailable`

Failure to spawn `ddcutil` with `ErrorKind::NotFound` produces `Unavailable`.
Automatic retries stop because repeatedly spawning a missing program cannot
recover. Manual Rescan remains permitted so installing `ddcutil` does not
require restarting Fang.

### `Cancelled`

`Cancelled` exists only for daemon shutdown. The runner performs complete
kill/reap/drain cleanup, the supervisor exits, and policy state is not updated.

## DDC Supervisor

### Ownership and publication

One asynchronous task owns:

- the DDC cache;
- `DdcDiscoveryPolicy`;
- the automatic deadline;
- the active operation future and its cancellation sender;
- one pending-manual-rescan latch;
- a bounded command receiver;
- publication of DDC cache and health.

The command channel has capacity 16 and uses fail-fast admission at the socket
boundary. A full broker returns `DDC supervisor busy` instead of waiting behind
hardware work.

The DDC cache fields, `monitor_present`, and `ddc_health` live in one published
snapshot protected by one lock. Every transition updates that snapshot in one
critical section, clones the completed snapshot, and builds `StateChanged`
from that same clone. A client can therefore never observe `ready` without the
corresponding monitor data, or stale controls after cache invalidation.

GPU snapshot updates use the same lock and update only GPU fields, preventing
lost writes between the two peripheral owners.

### Startup and active-operation loop

DDC discovery no longer runs inside `Peripherals::open`. Socket serving and
thermal sampling start first. In real-hardware mode the supervisor publishes
`searching/settling` and arms a five-second awake-time settle deadline. Manual
Rescan during settling cancels that deadline and starts discovery immediately.
Mock mode publishes a deterministic ready monitor without spawning a helper.

The supervisor uses an explicit biased selection order:

1. shutdown notification;
2. active-operation completion;
3. incoming command or resume notification;
4. automatic timer.

This order makes completion update policy before a queued command is checked.
If a manual command and timer become ready together, the manual command wins
and the scan origin is `manual`. A generation token invalidates stale timer
wakeups whenever a deadline changes.

While a discovery future is active, another manual request receives an
immediate `joined_active_scan` acknowledgment and starts no process. Its arrival
does not change the active scan's origin. While a setting future is active, the
first manual request sets the pending-manual latch and receives
`queued_after_active_operation`; later requests coalesce onto that latch.

Setting commands never queue behind another DDC operation. They are rejected
immediately as busy. Every command is permission-checked when removed from the
channel and again immediately before a helper is spawned. This second check is
mandatory after an operation completion, so a command already waiting in the
channel cannot run after the preceding scan opened the circuit.

### Shutdown

The main daemon sends the supervisor `Shutdown` and awaits its handle; it does
not abort the task. The supervisor:

1. stops accepting commands and disarms timers;
2. signals cancellation to the active operation, if any;
3. waits for process-group kill, direct-child reap, and both pipe drains;
4. rejects any pending setting responder with `daemon stopping`;
5. exits.

Only after the supervisor finishes does normal daemon shutdown complete. This
prevents a `ddcutil` descendant from surviving Fang.

## Pure Discovery Policy and Clock

The policy contains no subprocess, Tokio, UI, or hardware code. It consumes a
`DdcClock` timestamp, origin, trigger, and typed outcome, then returns the next
state, counters, deadlines, and permissions.

Production Linux builds implement `DdcClock` directly with
`clock_gettime(CLOCK_MONOTONIC)`. All settle, backoff, guard, operation, and
circuit deadlines therefore measure awake time and pause during suspend.
Tests inject a fake clock.

Tokio sleeps are only wakeup mechanisms. On every wake the supervisor checks
the policy clock; if awake time has not reached the deadline, it rearms the
sleep. This preserves the pause-during-suspend rule even if Rust or Tokio
changes an internal timer implementation.

A small Linux resume detector samples both `CLOCK_BOOTTIME`, which includes
suspend, and `CLOCK_MONOTONIC`, which excludes it. An increase of at least
250 ms in the difference between those clocks emits `ResumeDetected` on the
next one-second supervisor heartbeat. This replaces reliance on the existing
coarse wall-clock-jump heuristic for DDC timing and is unaffected by NTP or a
manual wall-clock correction.

Resume does not consume any cooldown. The supervisor recomputes relative
durations from unchanged awake-time deadlines and publishes a fresh health
event within two seconds of resume so the frontend recalibrates its countdown.

## State, Counters, and Timing

### Visible state and phase

`state` is:

- `searching`;
- `ready`;
- `backoff`;
- `circuit_open`;
- `unavailable`.

`phase` is:

- `settling`;
- `scanning`;
- `applying`;
- `idle`.

Valid active combinations are `searching/settling`,
`searching/scanning`, and `ready/applying`. The published `settings_allowed`
flag is false for every non-idle phase.

Published permissions are authoritative:

- `manual_rescan_allowed` is true only when no helper is active, shutdown has
  not begun, and the global manual guard has expired. It is true during
  settling so the user can skip the initial delay. A request may still join an
  active scan even though this field is false because joining spawns nothing.
- `settings_allowed` is true only for `ready/idle` with a cached monitor and at
  least one exposed setting feature. Each individual control additionally
  requires its matching feature field.

### Counters

The policy stores:

- `not_found_count`, for the normal absence ladder;
- `consecutive_hard_failures`, for breaker opening;
- `automatic_deadline`, if automatic work is scheduled;
- `manual_guard_deadline`, if a manual hard failure was recent;
- `last_failure`, if a hard or missing-helper result is still relevant.

Counter rules are explicit:

- `Found` resets both counters.
- `NotFound` increments `not_found_count` and resets hard failures.
- A successful setting or `SoftFailure` resets both counters.
- `HardFailure` increments hard failures and resets `not_found_count`.
- `Unavailable` resets both counters.

`last_failure` is set or replaced by `HardFailure` and `Unavailable`. It remains
visible while waiting and during a recovery probe. It is cleared by `Found`,
`NotFound`, a successful setting, or `SoftFailure`. Starting a scan alone does
not clear it.

### No-monitor backoff

Consecutive `NotFound` results schedule:

1. 15 seconds;
2. 30 seconds;
3. 1 minute;
4. 2 minutes;
5. 5 minutes.

Further absence stays capped at five minutes. Manual scans participate in the
same ladder, but remain available between automatic attempts.

### Hard-failure backoff and circuit

After the first consecutive hard failure, automatic retry waits 30 seconds.
After the second, it waits two minutes. The third opens the circuit for 30
minutes. While open, automatic discovery and all settings are suppressed.

When the 30-minute awake-time cooldown expires, one automatic half-open scan
runs. `Found` closes to `ready`; `NotFound` proves the helper is healthy and
closes to normal absence backoff; `Unavailable` enters unavailable; and
`HardFailure` reopens for a fresh 30 minutes.

### Global manual guard

Every manually initiated scan that ends in `HardFailure` sets a global
30-second manual guard, regardless of the state from which it started or
whether that failure opens/reopens the circuit. The guard is shared by every
socket client and blocks starting another manual helper process. Requests
during an already active scan may still join it because joining starts no new
process.

Automatic and setting-origin hard failures do not set the manual guard.
Therefore, if an automatic failure opens the circuit, one manual attempt is
available immediately. If that attempt fails, the global guard begins.

The guard affects only manual triggers; it does not postpone an already
scheduled automatic deadline. A normal later outcome clears it, and otherwise
it expires after 30 seconds of awake time.

## Complete Transition Tables

The tables are normative. "Manual allowed" means a new process may start;
joining an existing scan is always process-free.

### Trigger admission and race handling

| Current state / phase | Trigger | Origin | Action | Published effect |
|---|---|---|---|---|
| `searching/settling` | Manual Rescan | manual | Cancel settle deadline and start scan | `searching/scanning`; manual false; settings false |
| `searching/settling` | Settle deadline | automatic | Start scan | `searching/scanning`; manual false; settings false |
| any idle state, guard expired | Manual Rescan | manual | Start scan immediately | `searching/scanning`; retain ready cache until outcome; manual false; settings false |
| any idle state, guard active | Manual Rescan | manual | Reject without helper | State unchanged; return remaining guard |
| `searching/scanning` | Manual Rescan | manual | Join active scan | State unchanged; immediate joined acknowledgment |
| any state / `applying` | Manual Rescan | manual | Set one pending-manual latch | State unchanged; later requests coalesce |
| `ready/idle`, feature present | Matching setting | client setting | Recheck permission and start one helper | Prior state with `applying`; manual may queue; settings false |
| any non-ready or non-idle state | Setting | client setting | Reject without helper | State unchanged |
| any state | Automatic timer with matching generation | automatic | Recheck deadline and permission; start scan if due | `searching/scanning` |
| any state | Stale or early timer | automatic | Discard or rearm | State unchanged |
| timer and manual both ready | both | manual wins | Start one manual scan; invalidate timer generation | One helper only |
| operation completion and command both ready | both | completion first | Apply outcome atomically, then recheck command | Command sees new permissions |
| any state | Resume detected | lifecycle | Preserve awake-time deadlines and republish relative durations | Cache unchanged; frontend countdown recalibrates |
| any state | Shutdown | lifecycle | Cancel active runner, kill/reap/drain, reject pending work, exit | No new helper |

### Scan completion

| Prior context | Origin | Outcome | Next state | Counters and deadlines | Cache / failure | Allowed operations |
|---|---|---|---|---|---|---|
| any active scan | any | `Found` | `ready/idle` | Both counters 0; no automatic deadline; guard cleared | Atomically replace cache; clear failure | Manual yes; settings only for exposed features |
| any active scan | any | `NotFound` | `backoff/idle`, reason `not_found` | Hard count 0; increment absence; arm ladder deadline; guard cleared | Clear cache and failure | Manual yes; settings no |
| any active scan | manual | hard #1 | `backoff/idle`, reason `hard_failure` | Hard 1; absence 0; auto +30s; manual guard +30s | Clear cache; set failure | Manual after guard; settings no |
| any active scan | manual | hard #2 | `backoff/idle`, reason `hard_failure` | Hard 2; absence 0; auto +2m; manual guard +30s | Clear cache; set failure | Manual after guard; settings no |
| any active scan | manual | hard #3+ | `circuit_open/idle` | Increment hard; absence 0; auto +30m; manual guard +30s | Clear cache; set failure | Manual after guard; settings no |
| any active scan | automatic | hard #1 | `backoff/idle`, reason `hard_failure` | Hard 1; absence 0; auto +30s; no new guard | Clear cache; set failure | Manual immediately; settings no |
| any active scan | automatic | hard #2 | `backoff/idle`, reason `hard_failure` | Hard 2; absence 0; auto +2m; no new guard | Clear cache; set failure | Manual immediately; settings no |
| any active scan | automatic | hard #3+ | `circuit_open/idle` | Increment hard; absence 0; auto +30m; no new guard | Clear cache; set failure | Manual immediately; settings no |
| any active scan | any | `Unavailable` | `unavailable/idle` | Both counters 0; no automatic deadline; guard cleared | Clear cache; set missing failure | Manual yes; settings no |
| any active scan | lifecycle | `Cancelled` | supervisor exits | No policy update | Cleanup only | None |

### Setting completion

| Prior context | Outcome | Next state | Counters and deadlines | Cache / failure | Caller and operations |
|---|---|---|---|---|---|
| `ready/applying` | success | `ready/idle` | Both counters 0; no deadline; guard cleared | Update value; clear failure | Return updated status; settings allowed by feature |
| `ready/applying` | `SoftFailure` | `backoff/idle`, reason `soft_failure` | Both counters 0; auto +15s; guard cleared | Clear cache and failure | Return typed setting error; manual yes |
| `ready/applying` | hard #1 | `backoff/idle`, reason `hard_failure` | Hard 1; absence 0; auto +30s; no new guard | Clear cache; set failure | Return error; manual immediately |
| `ready/applying` | hard #2 | `backoff/idle`, reason `hard_failure` | Hard 2; absence 0; auto +2m; no new guard | Clear cache; set failure | Return error; manual immediately |
| `ready/applying` | hard #3+ | `circuit_open/idle` | Increment hard; absence 0; auto +30m; no new guard | Clear cache; set failure | Return error; manual immediately |
| `ready/applying` | `Unavailable` | `unavailable/idle` | Both counters 0; no automatic deadline; guard cleared | Clear cache; set missing failure | Return error; manual yes |
| any applying | lifecycle cancellation | supervisor exits | No policy update | Kill/reap/drain | Return `daemon stopping` if connected |

After any setting completion, a pending-manual latch is consumed only after the
new state is published and `manual_rescan_allowed` is recomputed. If permission
is no longer present, the latch is discarded without spawning.

## Protocol

The socket API version increases from 2 to 3. A newer app paired with an older
daemon must not imply that breaker protection exists. Read-only status remains
available across versions, while mutating commands require an exact API match.

`Status` gains:

```text
monitor_present: boolean
ddc_health: optional
  state: searching | ready | backoff | circuit_open | unavailable
  phase: settling | scanning | applying | idle
  backoff_reason: optional not_found | soft_failure | hard_failure
  retry_after_secs: optional integer
  manual_retry_after_secs: optional integer
  consecutive_hard_failures: integer
  manual_rescan_allowed: boolean
  settings_allowed: boolean
  last_failure: optional
    operation: DDC operation label
    kind: timed_out | signaled | output_truncated | runner_failed | missing
    signal: optional integer
```

In Rust, `ddc_health` is `Option<DdcHealth>` with `#[serde(default)]`.
`monitor_present` also has a default for read compatibility. A v3 daemon always
emits `Some`; missing health means an API v2/legacy daemon, not a current
`unavailable` state.

`backoff_reason` is present only for `backoff`. `retry_after_secs` is the
ceiling of the remaining automatic settle, retry, or circuit deadline and is
absent while scanning, ready, or unavailable. `manual_retry_after_secs` is the
ceiling of the independent global manual guard and is absent when that guard is
not active. Durations never become negative; a due deadline is processed
before publishing an idle zero.

On legacy status, the v3 UI:

- displays `DDC protection status is unavailable until the daemon is updated`;
- may display legacy monitor values read-only;
- disables Rescan and DDC settings because the API mismatch already forbids
  mutation;
- never invents breaker state, permissions, or retry durations.

The protocol never contains raw stderr. UI text is derived from typed fields.
The existing `color_ddc`, presets, current color, and brightness fields remain
for compatibility. `monitor_present` removes the need to overload
`color_ddc`.

### Rescan acknowledgment

`RescanDdc` no longer waits for discovery. It returns within the broker
acknowledgment budget with:

```text
accepted: boolean
disposition:
  started
  | joined_active_scan
  | queued_after_active_operation
  | denied_guard
  | denied_busy
manual_retry_after_secs: optional integer
```

A policy denial returns `accepted: false` without spawning and includes the
remaining guard when applicable. Transport, API-version, and full-broker
failures remain normal response errors. Scan progress and completion arrive
through `StateChanged`; the Tauri bridge must not replace its status store with
the acknowledgment object.

### End-to-end budgets

- Broker acknowledgment target: 250 ms; hard acknowledgment timeout: 1 second.
- Manual discovery request: acknowledgment only; no 35-second wait.
- A setting is admitted only while idle and runs one eight-second helper
  deadline. It is never queued behind another operation. With the required
  two-second cleanup acceptance bound, its supported end-to-end maximum is ten
  seconds.
- `GetStatus` reads the published snapshot and never waits for the supervisor
  or helper.
- The existing 35-second Tauri timeout remains safely above the maximum
  admitted setting duration, including local socket overhead.

## User Interface

The existing External Monitor card remains the only UI surface. No background
status polling is added.

For display only, the frontend derives countdowns from relative durations and
event receipt time. State events, especially `ResumeDetected` republishing,
recalibrate them. A frontend countdown never starts a DDC probe.

Messages are derived from state, phase, reason, and typed failure:

- settling: `Waiting for display hardware…`;
- scanning: `Looking for a DDC/CI monitor…`;
- ready with controls: existing brightness/color UI;
- ready without controls: `Monitor detected, but it did not report supported
  brightness or color controls.`;
- not-found backoff: `No DDC/CI monitor detected. Retrying in about …`;
- soft-failure backoff: `The monitor stopped responding over DDC/CI. Retrying
  in about …`;
- hard-failure backoff: `The DDC helper failed. Retrying in about …`;
- circuit open with a crash-class signal (`SIGSEGV`, `SIGABRT`, `SIGBUS`,
  `SIGILL`, or `SIGFPE`): `The DDC helper crashed repeatedly. Automatic checks
  are paused for …`;
- circuit open for any other cause, including timeout, non-crash signal,
  truncation, or runner failure: `The DDC helper failed repeatedly. Automatic
  checks are paused for …`;
- unavailable: `ddcutil is unavailable. Install it to enable monitor
  controls.`;
- legacy: `DDC protection status is unavailable until the daemon is updated.`;

The Rescan button:

- remains visible whenever health is known;
- is enabled only when `manual_rescan_allowed` is true;
- displays scan progress when `phase == scanning`;
- disables immediately after an accepted request and waits for state events;
- displays `manual_retry_after_secs` during the global guard.

Brightness and color controls require both their feature field and
`settings_allowed`. Errors remain local to the External Monitor card and never
imply a fan, thermal, network, or whole-daemon failure.

## Logging and Observability

The daemon logs one concise entry when:

- a hard failure occurs, including operation, origin, and typed reason;
- a retry or manual-guard delay changes;
- the circuit opens;
- a manual or automatic half-open probe starts;
- the circuit closes;
- `ddcutil` becomes unavailable;
- shutdown cancels and reaps a helper.

Repeated timer wakeups while open produce no log entry. Diagnostics are bounded
to the first 1 KiB, flattened to one line, and marked when truncated.

## Testing

### Process-runner tests

- successful output and normal nonzero exit;
- missing executable before a child exists;
- signal termination classification;
- direct timeout kills and reaps the complete process group;
- a direct child exits while a background descendant holds stdout/stderr open:
  the exit-plus-EOF deadline fires, the group is killed, and no process remains;
- a helper floods both streams beyond 64 KiB: both are drained without
  deadlock, both retained buffers are exactly bounded, and both truncation flags
  are set;
- DDC classifies either truncation flag as hard;
- cancellation kills/reaps/drains before returning;
- pipe and wait errors take the same cleanup path.

### Policy tests

Using a fake `DdcClock`, cover every normative transition-table row, including:

- the no-monitor ladder and five-minute cap;
- hard delays and opening on the third consecutive hard failure;
- all mixed `NotFound`/hard/soft counter resets;
- last-failure set, retention, replacement, and clear rules;
- manual Rescan during the settle period;
- manual Rescan during first- and second-hard backoff;
- immediate manual recovery after an automatic open;
- a 30-second guard after every manual-origin hard failure in every state;
- sequential requests from multiple clients during that guard start zero
  helpers;
- at guard expiry only one scan starts and simultaneous requests join it;
- automatic half-open after 30 awake minutes;
- suspend does not consume settle, guard, backoff, or circuit time;
- a `CLOCK_BOOTTIME`/`CLOCK_MONOTONIC` offset increase republishes health after
  resume without treating a wall-clock correction as resume;
- unavailable and legacy behavior.

### Supervisor and DDC-sequence tests

- startup and socket readiness do not wait for DDC;
- hard failure at every DDC stage aborts all later commands;
- optional-feature nonzero exits still yield a present monitor;
- a monitor with neither feature is ready, present, and settings-disabled;
- first valid display selection is deterministic with multiple displays;
- automatic/manual timer races follow the specified priority;
- simultaneous scan requests coalesce;
- sequential scan spam observes the global guard;
- setting requests during active work are rejected, not queued;
- a command already in the broker is permission-checked again after a preceding
  scan opens the circuit;
- a pending manual latch is rechecked after setting completion;
- cache and health publish atomically in one `StateChanged`;
- shutdown awaits runner cancellation and leaves no child;
- mock mode starts deterministic and ready.

### Protocol and integration tests

- API version 3 rejects mismatched mutating clients;
- API v2 JSON without `ddc_health` deserializes as `None`;
- pushed legacy `StateChanged` events deserialize successfully in the Tauri
  client;
- v3 status contains `monitor_present` and every health field;
- Rescan returns an acknowledgment rather than a status object;
- `GetStatus` remains responsive during a blocked fake helper.

### UI test strategy

Move health-to-copy, permission, and countdown derivation into a pure
`ddc-health.js` view-model module. Node's existing built-in test runner covers:

- every state/phase/reason message;
- generic versus signal-supported crash wording;
- known, unavailable, and legacy health;
- Rescan and settings enablement from daemon permission fields;
- accepted/rescan event flow without storing the acknowledgment as status;
- countdown recalibration after a simulated resume event;
- ready monitor with zero, one, or both features.

The Svelte component consumes that tested view model. Production `vite build`
continues to validate component compilation. CI adds `npm test` before
`npm run build`; the current workflow does not run the existing Node tests.

The workspace, daemon integration, Tauri, and UI suites must remain green.

## Measurable Hardware Acceptance

On the machine where `ddcutil 1.4.1` currently segfaults, and with a controlled
hanging/flooding helper where needed:

1. The Fang socket is listening and thermal sampling begins before the
   five-second DDC settle deadline.
2. During a hung DDC operation, 100 consecutive `GetStatus` requests have
   p95 latency below 250 ms and no request exceeds one second.
3. During ten minutes of DDC failure testing, the one-second telemetry stream
   has no gap greater than 2.5 seconds.
4. `SIGSEGV`, timeout, and truncation appear as their typed failures.
5. No later discovery stage runs after a hard failure.
6. Automatic retries follow 30 seconds, two minutes, then a 30-minute circuit
   cooldown measured in awake time.
7. Sequential manual requests cannot start hard-failing helpers less than 30
   awake seconds apart; simultaneous requests start one helper.
8. Suspend does not consume the circuit cooldown, and a health event within
   two seconds after resume restores the correct UI countdown.
9. Within two seconds after timeout cleanup or completed daemon shutdown,
   neither the direct helper PID nor any member of its process group exists.
10. Fan control, EC protection, GPU status, and the pre-test network interface
    state remain available throughout; Fang invokes no network-management
    command.

## Rollout

- Ship app and daemon together because the API version changes.
- Add `tokio`'s process feature to `fangd`.
- Add DDC safety behavior and legacy-status wording to the changelog and
  hardware-testing guide.
- Run Node UI tests in CI before the production build.
- Do not alter or upgrade the system `ddcutil` package automatically.
- Preserve existing uninstall and daemon restore behavior.
