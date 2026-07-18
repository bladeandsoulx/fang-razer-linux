# DDC Discovery Backoff and Circuit Breaker

**Date:** 2026-07-18
**Status:** Approved design
**Target:** Fang daemon, protocol, and External Monitor UI

## Problem

`fangd` currently starts `ddcutil` automatically during peripheral discovery and
then retries discovery at a fixed 15-second interval while no monitor is cached.
On the affected machine, the installed `ddcutil 1.4.1` process has repeatedly
terminated with `SIGSEGV`. The daemon survives, but every retry can probe the
display/GPU stack again, produce another crash, and delay daemon startup.

The current DDC wrapper collapses crashes, timeouts, unsupported features, and
ordinary "no monitor" results into the same unavailable result. It therefore
cannot slow down normal absence independently from protecting the machine
against a failing helper.

## Goals

- Preserve automatic external-monitor discovery.
- Use bounded exponential backoff when no compatible monitor is connected.
- Detect helper crashes and timeouts as hard failures.
- Stop automatic DDC work after repeated hard failures.
- Allow guarded manual recovery immediately and one automatic recovery probe
  after a long cooldown.
- Serialize every `ddcutil` invocation so scans and setting changes cannot race.
- Keep fan, thermal, socket, GPU, and EC control independent from DDC failures.
- Show users why DDC is waiting or paused and when it can retry.
- Make all policy behavior testable without real monitor hardware.

## Non-goals

- Replacing or repairing `ddcutil`.
- Adding udev/DRM hotplug listeners.
- Moving DDC into a second systemd service.
- Persisting circuit-breaker state across daemon restarts.
- Changing internal-panel brightness or refresh-rate behavior.
- Changing GPU-mode switching.

## Selected Approach

Use one daemon-local DDC supervisor backed by a pure
`DdcDiscoveryPolicy` state machine. Automatic timers, manual rescans,
brightness changes, and color changes all pass through the supervisor. The
supervisor owns the live `Ddc` object and is the only component allowed to
start `ddcutil`.

This provides a single safety boundary without new services or platform event
dependencies.

## Components

### Typed process outcome

The bounded helper runner will preserve structured failure information instead
of returning only strings. It must distinguish:

- spawn failure;
- wait or pipe failure;
- timeout, after killing and reaping the helper process group;
- normal exit with an exit code;
- signal termination on Unix;
- successful exit.

Existing non-DDC callers may map the typed error back to their current user
messages. DDC uses the structured form to classify outcomes safely. Captured
stdout and stderr remain bounded for diagnostics and must never be copied
unbounded into protocol state or logs.

### DDC command runner

DDC will call a small injectable command-runner interface. Production uses the
bounded process runner. Tests use a scripted fake that returns success,
nonzero exit, timeout, missing executable, or signal termination without
spawning real processes.

Every DDC operation is labeled for diagnostics:

- module preparation;
- display detection;
- capabilities;
- color read;
- brightness read;
- color write;
- brightness write.

`modprobe i2c-dev` remains best-effort. A normal nonzero result is not a hard
DDC failure because the module can already be built in or loaded.

### DDC supervisor

One asynchronous supervisor task owns:

- the `Ddc` cache;
- the discovery policy;
- the automatic retry timer;
- a command channel for manual rescan, color write, and brightness write;
- publication of the cached peripheral snapshot and health state.

DDC discovery no longer runs inside `Peripherals::open`. The supervisor begins
its first automatic scan after a five-second hardware-settle delay. DDC must
not delay thermal control or socket readiness.

Commands are single-flight. A manual rescan received during an active scan
joins that scan and receives its final status instead of starting another
helper. Setting commands are serialized behind an active scan.

### Pure discovery policy

The policy contains no subprocess, Tokio, UI, or hardware code. It consumes a
monotonic time and a typed operation outcome, then returns:

- the externally visible health state;
- whether an operation is currently permitted;
- the next automatic deadline;
- whether a manual probe is permitted;
- the consecutive hard-failure count.

Production scheduling uses monotonic time so wall-clock corrections and
suspend/resume do not create stale absolute deadlines. Protocol retry values
are relative durations derived from the policy snapshot.

## Outcome Classification

### Successful discovery

`Found` means a compatible monitor was detected and the discovery sequence
completed without a hard helper failure. Optional features may be absent.
Normal nonzero exits from `capabilities` or `getvcp` mean that feature is not
available; they do not make an otherwise safe monitor discovery fail.

### Normal absence

`NotFound` means `ddcutil` ran normally but no compatible display was found.
This is expected when a monitor is unplugged and never contributes to the
circuit-breaker failure count.

### Hard failure

The following are hard failures:

- timeout;
- termination by a signal, including `SIGSEGV`;
- failure while waiting for or collecting a spawned helper;
- an equivalent internal runner failure after the child started.

A hard failure aborts the current DDC sequence immediately. No later
capability, read, or write command may run during that sequence.

### Dependency unavailable

Failure to spawn `ddcutil` because the executable is absent produces
`Unavailable`. Automatic retries stop because repeatedly spawning a missing
program cannot recover. Manual Rescan remains available so installing
`ddcutil` does not require a daemon restart.

Other spawn failures are reported as bounded diagnostic failures and treated
as hard failures unless they clearly mean the executable is absent.

### Soft operation failure

A normal nonzero exit during a read or write, such as an unsupported VCP,
monitor-busy response, or disabled DDC/CI setting, is a soft operation failure.
It is returned to the user and may invalidate the cached monitor. Because the
helper completed normally, it also ends any consecutive hard-failure streak and
resets the hard-failure count.

## State Machine

The visible states are:

- `searching`: a scan is in progress or the initial settle delay is active;
- `ready`: a compatible monitor is cached;
- `backoff`: no monitor was found or a pre-threshold hard failure occurred;
- `circuit_open`: repeated hard failures paused DDC operations;
- `unavailable`: the `ddcutil` executable could not be found.

### No-monitor backoff

The first `NotFound` schedules the next scan after 15 seconds. Consecutive
`NotFound` outcomes use:

1. 15 seconds;
2. 30 seconds;
3. 1 minute;
4. 2 minutes;
5. 5 minutes.

Further absence remains capped at five minutes. `NotFound` resets the
consecutive hard-failure counter because the helper completed normally.

### Hard-failure backoff

Hard failures must not occur in a tight loop:

1. the first consecutive hard failure retries after 30 seconds;
2. the second retries after 2 minutes;
3. the third opens the circuit for 30 minutes.

`Found` and `NotFound` both reset the consecutive hard-failure counter. A soft
operation failure does not increment it.

### Open and half-open behavior

While open:

- automatic discovery is suppressed;
- color and brightness reads/writes are rejected without spawning a helper;
- one manual Rescan is immediately permitted;
- after 30 minutes, one automatic half-open discovery probe is permitted.

Only one half-open probe can run at a time.

- `Found` closes the breaker and returns to `ready`.
- `NotFound` proves the helper is healthy, closes the breaker, and enters the
  normal no-monitor backoff.
- `Unavailable` enters the unavailable state.
- A hard failure reopens the circuit for a fresh 30-minute cooldown.

After a manual half-open probe hard-fails, further manual probes are blocked
for 30 seconds to prevent click-spamming. This short manual guard does not
apply when the circuit was first opened by automatic retries, so the promised
immediate manual recovery attempt remains available.

Breaker state is intentionally in memory. A reboot or deliberate daemon
restart permits a fresh initial scan and naturally recovers after a helper
upgrade without leaving a stale persisted lockout.

## Discovery Sequence

A discovery scan performs these stages in order:

1. best-effort `modprobe i2c-dev`;
2. `ddcutil detect`;
3. capabilities for the selected external display;
4. current color preset;
5. current brightness.

Any hard failure at stages 2–5 stops the sequence immediately. Normal nonzero
results at optional stages 3–5 leave that feature absent and allow discovery
to complete safely.

Once a monitor is `ready`, automatic discovery stops. A soft write failure
invalidates the cached monitor and schedules normal rediscovery. A hard read or
write failure immediately invalidates the cached monitor, enters the same
hard-failure backoff policy, and may open the breaker.

## Protocol

The socket API version increases from 2 to 3. A newer app paired with an older
daemon must not imply that circuit-breaker protection exists when it does not.
The existing compatibility behavior remains: read-only status is available,
but hardware-changing commands require an exact API match.

`Status` gains a DDC health object with serde defaults for robust read-only
deserialization:

```text
ddc_health:
  state: searching | ready | backoff | circuit_open | unavailable
  retry_after_secs: optional integer
  manual_retry_after_secs: optional integer
  consecutive_hard_failures: integer
  last_failure:
    operation: optional DDC operation label
    kind: timed_out | signaled | runner_failed | missing
    signal: optional integer
```

The protocol never includes raw stderr. The daemon may include a bounded,
single-line helper diagnostic in its journal, but UI text is derived from the
typed fields.

The existing `color_ddc`, preset, color, and brightness fields remain. Existing
state-change events publish whenever DDC availability or health changes.

## User Interface

The existing External Monitor card remains the only UI surface. No new screen
or background polling is added.

For display only, the frontend derives a local countdown from each relative
retry duration and the event receipt time. State-change events recalibrate that
countdown; it never drives daemon policy or starts a DDC probe.

It displays health-specific text:

- searching: `Looking for a DDC/CI monitor…`;
- ready: existing brightness and color controls;
- backoff: `No compatible monitor found. Retrying in about 2 minutes.`;
- circuit open: `DDC helper crashed repeatedly. Automatic checks are paused
  for 30 minutes.`;
- unavailable: `ddcutil is unavailable. Install it to enable monitor
  controls.`;

The existing Rescan button:

- remains visible whenever DDC is not ready;
- shows progress for the active single-flight scan;
- triggers the permitted manual half-open probe while the circuit is open;
- is disabled only during an active scan or the 30-second post-manual-failure
  guard;
- shows the remaining short guard duration when temporarily disabled.

Brightness and color controls are hidden or disabled when the circuit is open.
Errors remain local to the External Monitor card and must not imply a fan,
thermal, or daemon failure.

## Logging and Observability

The daemon logs one concise entry when:

- a hard failure occurs, including the operation and typed reason;
- the retry delay changes;
- the circuit opens;
- a manual or automatic half-open probe starts;
- the circuit closes;
- `ddcutil` is unavailable.

Repeated timer ticks while the circuit is open produce no log entry. Helper
stderr is truncated and flattened to a single line before journaling.

## Testing

### Process tests

- successful output capture;
- normal nonzero exit remains an `Output`;
- timeout kills and reaps the complete process group;
- a child that sends itself `SIGSEGV` is classified as signaled;
- missing executable is classified separately;
- captured diagnostics are bounded.

### Policy tests

Using a fake monotonic clock:

- the full no-monitor backoff ladder and five-minute cap;
- hard-failure delays of 30 seconds and 2 minutes;
- opening after the third consecutive hard failure;
- reset on `Found`;
- reset and normal backoff on `NotFound`;
- reset on soft failures because the helper completed normally;
- automatic half-open after 30 minutes;
- immediate manual half-open while open;
- 30-second guard only after a failed manual half-open attempt;
- close/reopen behavior for every half-open outcome;
- unavailable state and manual recovery.

### DDC sequence tests

With a scripted command runner:

- successful full discovery;
- safe discovery with unsupported optional features;
- no-monitor result;
- hard failure at every stage aborts all later commands;
- soft write failure invalidates the cache without opening the breaker;
- hard read/write failure invalidates the cache and contributes to the breaker;
- operations are rejected without a subprocess while open;
- missing `ddcutil` stops automatic scheduling.

### Supervisor and protocol tests

- initial scan starts after the settle delay without blocking daemon readiness;
- automatic and manual triggers remain single-flight;
- simultaneous manual requests coalesce onto one scan;
- cached snapshot and health changes emit state events;
- API version 3 rejects mismatched mutating clients;
- mock mode remains deterministic and starts `ready`;
- integration status includes every DDC health field.

### UI tests

- every health state renders the correct message;
- retry and manual-guard durations render safely;
- Rescan availability follows policy state;
- ready controls disappear or disable when the circuit opens;
- a manual request cannot be submitted twice while active.

The existing workspace, daemon integration, Tauri, and UI test suites must
remain green, and the production UI build must succeed.

## Hardware Acceptance

On the machine where `ddcutil 1.4.1` currently segfaults:

1. Fang starts and its socket, thermal sampling, and fan protection remain
   responsive.
2. A `SIGSEGV` is reported as a typed hard failure.
3. No later command from that discovery sequence runs after the crash.
4. Automatic retries use the specified hard-failure delays.
5. The third consecutive hard failure opens the circuit.
6. No automatic `ddcutil` process starts during the 30-minute open period.
7. Manual Rescan is immediately available once the circuit opens.
8. A failed manual probe triggers the 30-second manual guard and restarts the
   long cooldown.
9. One automatic half-open probe runs after the long cooldown.
10. Fan, EC, GPU, socket, and network behavior are unaffected throughout.

## Rollout

- Ship the app and daemon together because the API version changes.
- Add the DDC safety behavior to the changelog and hardware-testing guide.
- Do not automatically alter or upgrade the system `ddcutil` package.
- Preserve the existing uninstall and daemon restore behavior.
