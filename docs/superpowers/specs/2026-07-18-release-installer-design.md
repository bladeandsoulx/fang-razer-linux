# Fang release installer and immutable publication

**Date:** 2026-07-18
**Status:** Approved

## Goal

Give Fang users a safe, friendly installation and upgrade path that requires
only one command:

```sh
curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash
```

The installer selects the correct DEB or RPM pair, verifies it, installs the
desktop app and daemon together, activates `fangd`, and grants the invoking
desktop user access to the `fang` group. It supports the release-tested
Ubuntu, Debian, and Fedora versions plus clearly identified compatible-family
derivatives such as Zorin, Mint, and Pop!_OS.

The release pipeline publishes the installer and packages as one immutable,
six-asset release. Every install is therefore locked to the exact tag and
artifacts selected when that release was published.

## Current-state constraint

`v0.9.3` was published on 2026-07-18 with four package assets and without
release immutability enabled. It must not be retrofitted with the installer or
checksum manifest because doing so would violate this design's publish-once
contract.

Release immutability applies only to future releases. The first
installer-enabled release is therefore `v0.9.4`, assuming the normal next
version, or whichever later version is current when this work ships. Examples
in this document use `0.9.4`; implementation derives and verifies the actual
version from the repository manifests and tag.

## Scope

### In scope

- One release-locked Bash installer distributed as `install.sh`.
- A friendly terminal banner and concise progress, warning, and error output.
- Native DEB installation on supported Ubuntu and Debian bases.
- Native RPM installation on Fedora 43 and Fedora 44.
- Compatible-family derivative detection through safely parsed
  `/etc/os-release` data.
- Exact package-name, version, release, and architecture validation.
- SHA-256 verification of the two packages selected for the current system.
- Native DEB and RPM installed-version comparison.
- Fresh installs, safe upgrades, and idempotent same-version repair.
- Daemon service activation and desktop-user group reconciliation.
- Refusal of downgrades, root invocation, unsupported bases, and unsupported
  architectures before elevation.
- A six-asset immutable GitHub release assembled as a draft and published
  exactly once.
- DEB, RPM, installer-fixture, ShellCheck, checksum, metadata, and release
  inventory gates.
- An inspect-before-running installation path in the README.
- Renaming the existing source-build script to
  `packaging/install-from-source.sh`.

### Out of scope

- Modifying or replacing the existing `v0.9.3` release.
- aarch64, ARM, i386, or architectures other than x86_64/amd64.
- RHEL, CentOS Stream, Rocky Linux, AlmaLinux, openSUSE, Arch, NixOS, or
  distributions outside the declared DEB and Fedora families.
- Ubuntu 20.04, Fedora 42, Debian 11, untested future base versions, or
  ambiguous derivative bases.
- A `--force`, unsupported-platform, or downgrade override.
- Direct root invocation or an explicit `--target-user` mode.
- COPR, APT repositories, DNF repositories, or background automatic updates.
- Package signing in this iteration. The existing direct-download RPMs remain
  unsigned.
- An interactive package, component, or destination selector.
- Automatically logging out, restarting the desktop session, or rebooting.
- Uninstallation through the installer.

## Supported-system policy

### Architecture

The installer accepts only `uname -m` equal to `x86_64`.

- The corresponding DEB architecture is exactly `amd64`.
- The corresponding RPM architecture is exactly `x86_64`.
- Any other value is rejected before downloads or elevation.

The installer never silently maps a different architecture to x86_64.

### Release-tested direct distributions

The initial release-tested matrix is:

| Family | Direct distribution | Accepted base versions |
| --- | --- | --- |
| DEB | Ubuntu | 22.04 (`jammy`), 24.04 (`noble`) |
| DEB | Debian | 12 (`bookworm`), 13 (`trixie`) |
| RPM | Fedora | 43, 44 |

These boundaries are an allowlist, not a minimum-version comparison. For
example, Ubuntu 26.04 is not accepted merely because it is newer than 24.04.
Adding a base version requires adding its install test first.

### Compatible-family derivatives

Derivatives proceed automatically only when both their family and underlying
base can be identified without guessing. They receive a warning that the
family is compatible but the derivative is not release-tested directly.

The installer safely parses only the needed keys from `/etc/os-release`; it
must not `source`, `eval`, or otherwise execute the file. Quoted values are
decoded as data using the `os-release` quoting rules, without command or
parameter expansion. Duplicate relevant keys, malformed quoting, or values
outside the field's expected character set are rejected rather than resolved
by last-one-wins behavior. The relevant fields are:

- `ID`
- `ID_LIKE`
- `VERSION_ID`
- `VERSION_CODENAME`
- `UBUNTU_CODENAME`
- `PLATFORM_ID`

Selection rules are evaluated in this order:

1. `ID=ubuntu` is accepted only for `VERSION_ID=22.04` or `24.04`.
2. `ID=debian` is accepted only for `VERSION_ID=12` or `13`.
3. `ID=fedora` is accepted only for `VERSION_ID=43` or `44`.
4. A derivative whose `ID_LIKE` contains `ubuntu` is accepted only when
   `UBUNTU_CODENAME` is `jammy` or `noble`.
5. A non-Ubuntu derivative whose `ID_LIKE` contains `debian` is accepted only
   when its base marker resolves explicitly to `bookworm` or `trixie`.
   `VERSION_CODENAME` may be used only when it is one of those exact base
   codenames.
6. A derivative whose `ID_LIKE` contains `fedora` is accepted only when
   `PLATFORM_ID` is exactly `platform:f43` or `platform:f44`.
7. Conflicting, missing, unrecognized, or out-of-range markers are rejected.

For example, Zorin OS 18.1 reports `ID=zorin`, an Ubuntu-compatible
`ID_LIKE`, and `UBUNTU_CODENAME=noble`. It selects the DEB pair automatically
and prints:

```text
✓ Detected: linux (zorin → Ubuntu 24.04 family)
! Zorin is compatible-family, not release-tested directly.
```

Mint and Pop!_OS follow the same rule when they expose a supported
`UBUNTU_CODENAME`. Their derivative version number is never compared directly
with an Ubuntu version number.

## Invocation and privilege boundary

The documented command is run as the logged-in desktop user. It must not be
prefixed with `sudo`.

At startup the installer:

1. Refuses `EUID=0` with a message to rerun as the desktop user without
   `sudo`.
2. Captures the invoking username and numeric UID from `id`, before any
   elevation.
3. Resolves that user's home through `getent passwd`; it does not trust
   `$HOME`, `$USER`, or `$SUDO_USER` as identity.
4. Rejects an empty, root, or unresolvable target identity.

The installer performs all of the following without sudo:

- platform and architecture detection;
- prerequisite checks;
- temporary-directory creation;
- manifest and package downloads;
- checksum verification;
- package metadata validation;
- installed-package queries; and
- downgrade decisions.

Only after all checks succeed may it call `sudo -v`, followed by the package
transaction, service reconciliation, and group reconciliation. A validation
failure or detected downgrade therefore cannot prompt for a password or
change the system.

`sudo` normally reads credentials from the controlling terminal, so the
documented `curl | bash` pipeline remains usable without consuming package
input from standard input. No other installer step prompts for input.

## Streaming-safe script structure

The release installer is a repository-root `install.sh`. The existing
source-build script moves from `packaging/install.sh` to
`packaging/install-from-source.sh`, and its README command changes with it.

`install.sh` contains, in order:

1. the Bash shebang;
2. comments;
3. function definitions; and
4. one sole `main "$@"` invocation as the final executable line.

All initialization, strict-mode setup, constant assignment, trap
registration, validation, downloads, and mutations happen inside functions
called by `main`. There are no top-level downloads, package commands, sudo
calls, or partial installation steps.

This layout gives the streaming command a fail-closed truncation property:

- truncation inside a function definition is a parse error;
- truncation after complete function definitions but before the final line
  never calls `main`; and
- only receipt of the final invocation begins the installation flow.

`main` enables strict error behavior, sets a private umask, registers cleanup,
and then dispatches the workflow. Temporary files are removed on success,
failure, and catchable `INT`, `HUP`, and `TERM` signals. The design does not
claim cleanup after `SIGKILL` or host failure.

The script requires Bash and does not claim POSIX `sh` compatibility.

## Release locking and exact artifact selection

The public bootstrap URL uses GitHub's official latest-release asset form:

```text
https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh
```

Each uploaded installer is release-locked. It embeds constants for:

- repository owner and name;
- exact release tag;
- application version;
- exact DEB daemon filename and expected metadata;
- exact DEB desktop filename and expected metadata;
- exact RPM daemon filename and expected metadata; and
- exact RPM desktop filename and expected metadata.

After the latest URL supplies the installer, every subsequent download uses
the embedded direct tag URL:

```text
https://github.com/bladeandsoulx/fang-razer-linux/releases/download/v0.9.4/ASSET
```

The installer does not call the GitHub API, inspect release HTML, query the
latest version, use `jq`, discover filenames with globs, or combine assets
from different releases.

Future version bumps must update or render these constants from the same
version source used by the package build. The version-sync check verifies each
constant and filename before a tag can release.

## Exact six-asset release contract

For the first installer-enabled release, shown here as `v0.9.4`, the manually
attached release assets are exactly:

1. `install.sh`
2. `SHA256SUMS`
3. `Fang_0.9.4_amd64.deb`
4. `fangd_0.9.4-1_amd64.deb`
5. `fang-0.9.4-1.x86_64.rpm`
6. `fangd-0.9.4-1.x86_64.rpm`

GitHub-generated source archives are not manually attached assets and are not
part of the six-entry API inventory. GitHub's automatically generated release
attestation is likewise not a seventh manually attached asset.

The general filename formulas are:

| Artifact | Exact filename |
| --- | --- |
| Installer | `install.sh` |
| Checksum manifest | `SHA256SUMS` |
| DEB desktop | `Fang_${VERSION}_amd64.deb` |
| DEB daemon | `fangd_${VERSION}-${DEB_REVISION}_amd64.deb` |
| RPM desktop | `fang-${VERSION}-${FANG_RPM_RELEASE}.x86_64.rpm` |
| RPM daemon | `fangd-${VERSION}-${FANGD_RPM_RELEASE}.x86_64.rpm` |

The initial revisions/releases are:

- DEB desktop version: `0.9.4`
- DEB daemon version: `0.9.4-1`
- RPM desktop: Version `0.9.4`, Release `1`, Epoch absent/zero
- RPM daemon: Version `0.9.4`, Release `1`, Epoch absent/zero

The DEB version and RPM EVR expectations are independent values. The workflow
must not assume that a DEB revision and RPM Release are interchangeable or
that the desktop and daemon always share identical package versions. No
`sort -V`, filename prefix match, or wildcard can stand in for package
metadata.

## Checksum contract

`SHA256SUMS` contains exactly five entries:

1. `install.sh`
2. `Fang_0.9.4_amd64.deb`
3. `fangd_0.9.4-1_amd64.deb`
4. `fang-0.9.4-1.x86_64.rpm`
5. `fangd-0.9.4-1.x86_64.rpm`

It cannot include its own digest. Each line uses the canonical
`sha256sum` text form:

```text
LOWERCASE_64_HEX_DIGEST␠␠BASENAME
```

The manifest has:

- one final newline;
- no path components;
- no duplicate filenames;
- no missing expected filename; and
- no extra filename.

Release CI regenerates and verifies all five digests before upload. It also
validates the uploaded manifest byte-for-byte and checks the complete
six-asset inventory before publication.

The installer validates the manifest shape and exact expected filename set,
extracts only the two package entries selected for the current system, and
uses `sha256sum -c` to verify those two local files. It does not download or
rehash the two packages for the other family.

In pipe mode, `install.sh` cannot authenticate its own bytes before running.
The installer checksum exists for the inspect-first path and release
inventory. Package checksums prevent corruption and cross-asset mix-ups, but
the manifest is stored in the same release as the packages. Release
immutability and its generated release attestation are the stronger
anti-replacement boundary.

## Package metadata validation

Checksums are necessary but not sufficient: the installer validates the
selected package pair's internal metadata before sudo.

### DEB metadata

`dpkg-deb -f` must report:

| File | Package | Version | Architecture |
| --- | --- | --- | --- |
| `Fang_0.9.4_amd64.deb` | `fang` | `0.9.4` | `amd64` |
| `fangd_0.9.4-1_amd64.deb` | `fangd` | `0.9.4-1` | `amd64` |

### RPM metadata

`rpm -qp` must report:

| File | Name | Epoch | Version | Release | Architecture |
| --- | --- | --- | --- | --- | --- |
| `fang-0.9.4-1.x86_64.rpm` | `fang` | absent or `0` | `0.9.4` | `1` | `x86_64` |
| `fangd-0.9.4-1.x86_64.rpm` | `fangd` | absent or `0` | `0.9.4` | `1` | `x86_64` |

Every field is checked independently against an embedded expected value. A
correct-looking filename with wrong internal metadata is rejected without
elevation.

## Download behavior

The installer creates a mode-0700 temporary directory under
`${TMPDIR:-/tmp}` with `umask 077`. Fixed local basenames are used; server
filenames and response headers never choose paths.

Internal downloads:

- require HTTPS for the initial and redirected protocols;
- use the embedded direct release tag;
- fail on HTTP errors;
- follow redirects;
- retry a small bounded number of transient failures;
- write to a temporary filename before moving it into its fixed final name;
  and
- leave no partial package accepted as complete.

The installer checks the required commands for its chosen family before
downloading:

- common: `curl`, `sha256sum`, `uname`, `id`, `getent`, `mktemp`,
  `systemctl`, and `sudo`;
- DEB: `dpkg`, `dpkg-deb`, `dpkg-query`, and `apt-get`;
- RPM: `rpm` and `dnf`.

Missing prerequisites cause a clear refusal. The installer does not elevate
early to install its own prerequisites.

## Installed-version and downgrade policy

The installer queries `fang` and `fangd` independently and classifies each as:

- absent;
- older than the selected release;
- equal to the selected release; or
- newer than the selected release.

For DEB, only `install ok installed` counts as installed; residual
configuration state counts as absent. If the RPM database returns more than
one installed EVR for either package name, the state is ambiguous and the
installer refuses before elevation rather than choosing one.

DEB comparisons use `dpkg --compare-versions` on the complete Debian package
version. RPM comparisons use RPM's own EVR semantics, including Epoch,
Version, and Release, through the installed RPM implementation
(`rpm.vercmp` exposed by RPM's Lua evaluation interface). The operands are
passed as data rather than interpolated as Lua source.

`sort -V`, lexical comparison, and hand-written semantic-version comparison
are forbidden.

The decision table is:

| Installed `fang` | Installed `fangd` | Action before sudo | Elevated action |
| --- | --- | --- | --- |
| newer | any | refuse downgrade | none |
| any | newer | refuse downgrade | none |
| equal | equal | accept idempotent run | skip package transaction; reconcile service and group |
| absent/older/equal | absent/older/equal, but not both equal | accept install/upgrade | submit both selected local packages in one native transaction |

Passing both package paths in one transaction preserves dependency resolution
and prevents an intentional app/daemon split:

```text
sudo apt-get install ... ABSOLUTE_FANGD_DEB ABSOLUTE_FANG_DEB
sudo dnf install ... ABSOLUTE_FANGD_RPM ABSOLUTE_FANG_RPM
```

The implementation supplies exact absolute paths, not `./` paths or globs.
The package manager may internally skip an already-equal member of a mixed
pair, but both files are presented in the same requested transaction.

If either installed package is newer, the installer names that package,
installed version, and selected release version, then exits before `sudo -v`.
It never attempts a partial downgrade to make the pair match.

## Service and group reconciliation

After a successful package transaction, or immediately after the equal/equal
idempotent path, the installer:

1. Confirms that the `fang` group exists.
2. Runs `sudo systemctl enable --now fangd`.
3. Confirms `fangd` is active.
4. Checks whether the captured desktop user is already a member of `fang`.
5. Runs `sudo usermod -aG fang CAPTURED_USER` only when membership is missing.

This deliberately reconciles both package families:

- the current DEB daemon package normally enables and starts the service in
  its package scripts;
- the Fedora package follows Fedora preset policy and does not force
  activation itself; and
- an equal/equal rerun repairs a disabled service or missing user membership
  without reinstalling packages.

The final message says to log out and back in once only when group membership
was newly added. It does not claim the current desktop session acquired the
new group.

If service startup fails, the installer exits nonzero and prints a bounded,
noninteractive `systemctl status fangd` diagnostic. It does not report
installation success merely because the package transaction completed.

## Terminal experience

On an interactive terminal, the installer begins with:

```text
┌─────────────────────────────────────────────────────────┐
│                  ◆ Fang Installer                       │
├─────────────────────────────────────────────────────────┤
│  Fan, power, lighting, and telemetry for Razer Blade.   │
└─────────────────────────────────────────────────────────┘
```

The checked-in output snapshot owns the exact spacing so the border remains
aligned.

Progress uses the same compact vocabulary throughout:

- `✓` completed step;
- `→` current step;
- `!` compatibility or session warning; and
- `✗` fatal error.

A successful Zorin upgrade resembles:

```text
✓ Detected: linux (zorin → Ubuntu 24.04 family)
! Zorin is compatible-family, not release-tested directly.
→ Downloading Fang 0.9.4 packages...
✓ Checksums and package metadata verified
✓ Installed Fang 0.9.4
✓ fangd is enabled and active
✓ Added home to the fang group
! Log out and back in once before launching Fang.
```

Colors are enabled only when standard output is a terminal and `NO_COLOR` is
unset. Noninteractive logs retain the words and symbols without escape
sequences. Errors always include the failed phase and a concrete next action;
raw package-manager output remains available rather than being hidden behind
the decorative status layer.

## Immutable release workflow

### One-time repository prerequisite

Before the first installer-enabled tag is pushed, a repository administrator
enables **Settings → Releases → Enable release immutability**, or uses
GitHub's authenticated immutable-releases API.

The tag workflow performs a read-only preflight against:

```text
GET /repos/bladeandsoulx/fang-razer-linux/immutable-releases
```

It fails before creating a draft unless the response confirms
`enabled: true`. This is a publication prerequisite, not a best-effort
warning.

### Gated publication sequence

The release workflow becomes:

```text
DEB build ──> DEB install tests ───────────────┐
RPM build ──> Fedora 43/44 install tests ──────┤
installer fixtures + ShellCheck ───────────────┤
version/package/release metadata checks ───────┤
immutable-setting preflight ───────────────────┘
                                                │
                                                v
                                      create one draft
                                                │
                                      attach six exact assets
                                                │
                                 validate remote inventory/digests
                                                │
                                      publish draft exactly once
                                                │
                                assert immutable release postcondition
```

The publication job:

1. Uses a per-tag concurrency group so duplicate workflow runs cannot race.
2. Verifies the tag matches all package and installer versions.
3. Stages the six exact basenames in an otherwise empty directory.
4. Generates and validates the five-line checksum manifest.
5. Refuses to continue if a release for the tag already exists.
6. Creates one draft release.
7. Uploads six explicitly named paths without globs or overwrite/clobber
   behavior.
8. Reads the draft through the releases API and compares the asset-name set,
   count, size, and available SHA-256 digest metadata with the local staged
   files.
9. Publishes the draft in one transition only after every check succeeds.
   Publication explicitly marks this non-prerelease as GitHub's latest
   release.
10. Reads the published release and asserts:
    - `draft` is false;
    - `prerelease` is false;
    - `immutable` is true;
    - the tag is the expected tag; and
    - the exact six assets remain present.
11. Resolves GitHub's latest-release endpoint and confirms that it identifies
    the new tag and its `install.sh`.

If a workflow fails after creating the draft but before publication, it
leaves the draft unpublished and fails closed. A maintainer must inspect and
delete that draft before rerunning; automation never silently updates,
clobbers, or publishes a pre-existing draft.

Once published, GitHub locks the release tag and assets and generates the
release attestation. No job or maintainer instruction attempts to append,
replace, rename, or delete an asset afterward.

Because the published release is non-prerelease and non-draft, GitHub's
`releases/latest/download/install.sh` route resolves to its installer once it
is the latest release.

## Test strategy

### Installer fixture tests

Tests execute the real `install.sh` entry point with isolated PATH stubs,
fixture package metadata, fixture `/etc/os-release` content, and a temporary
filesystem. They record attempted downloads, sudo calls, package
transactions, service calls, and group changes.

Required fixture cases include:

- the final `main` call is the only top-level executable installer action;
- every truncation boundary before the final invocation causes no mutation;
- direct root invocation is refused;
- the captured user is stable and not taken from `$USER`, `$HOME`, or
  `$SUDO_USER`;
- Ubuntu 22.04/24.04, Debian 12/13, and Fedora 43/44 direct detection;
- Zorin, Mint, and Pop!_OS supported-base derivative selection with warning;
- unsupported derivative bases, conflicting markers, future direct bases,
  and unknown distributions;
- representative non-x86_64 architectures plus rejection of arbitrary values
  other than exact `x86_64`;
- exact tag URLs and exact selected asset names;
- download failure and partial-file cleanup;
- missing, duplicate, malformed, extra, and incorrect checksum entries;
- checksum mismatch for each selected package;
- correct filename with wrong package name, version, release, epoch, or
  architecture;
- installed-version matrices covering absent, older, equal, newer, and mixed
  app/daemon states;
- Debian revision ordering that differs from `sort -V`;
- RPM Epoch/Version/Release ordering;
- downgrade refusal before any sudo call;
- no sudo call before all validation completes;
- both packages passed to one transaction on fresh and mixed-state paths;
- equal/equal skips package installation but repairs service and group;
- already-active/already-member idempotence;
- service-start failure diagnostics and nonzero exit;
- temporary-directory cleanup on success, failure, and signal;
- TTY color, `NO_COLOR`, and non-TTY output behavior; and
- the checked-in banner/output snapshot.

The fixture suite must not access GitHub, modify host packages, call real
sudo, or touch the host systemd instance.

### Shell validation

ShellCheck runs against:

- `install.sh`;
- `packaging/install-from-source.sh`;
- release staging/validation shell scripts; and
- existing RPM build and verification scripts.

Any intentional suppression is local, documented, and justified.

### DEB build and install tests

The DEBs continue to build on Ubuntu 22.04, the oldest supported Ubuntu
baseline. The exact resulting pair is tested in clean x86_64 containers for:

- Ubuntu 22.04;
- Ubuntu 24.04;
- Debian 12; and
- Debian 13.

Each test verifies:

- exactly two expected DEB filenames;
- exact internal package names, independent versions, architecture, and
  desktop-to-daemon dependency bounds;
- both local packages install together with dependencies in one transaction;
- installed versions match the artifacts;
- the `fang` group exists;
- the service unit is present and passes static verification;
- `fangd --version` and the existing mock daemon smoke test succeed;
- the desktop binary has no unresolved shared libraries;
- the installed desktop app stays alive for a bounded startup window under
  `dbus-run-session` and Xvfb, then is terminated by the test;
- package verification reports no missing packaged files; and
- both packages remove cleanly.

Containers that do not run systemd as PID 1 validate the unit and daemon
directly; they do not claim that a real service manager started the unit.

### RPM tests

The existing Fedora 43 build and Fedora 43/44 install-test matrix remains a
release gate. The installer work adds exact release-asset-name and installer
transaction fixtures but does not weaken the current RPM metadata,
dependency, sysusers, daemon, desktop, and removal checks.

### Release contract tests

Host-runnable tests cover:

- the exact six expected asset names for a fixture version;
- exact package metadata values for all four packages;
- a five-entry manifest that cannot contain itself;
- deterministic manifest order and canonical formatting;
- detection of a missing, duplicate, or seventh asset;
- rejection of filename globs in publication inputs;
- tag/version disagreement;
- an existing draft or published release;
- immutable-setting preflight failure; and
- draft inventory validation before the single publish command.

The tag workflow runs all DEB, RPM, installer, ShellCheck, checksum, metadata,
and inventory gates independently even if the same commit passed pull-request
CI.

## Documentation

The README leads with the one-line installer for supported systems:

```sh
curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash
```

Immediately below it, the README states:

- run it as the desktop user, not with `sudo`;
- it downloads and validates packages before asking for sudo;
- it installs or upgrades both components and refuses downgrades;
- derivatives receive a compatibility warning;
- only the declared x86_64 base versions are accepted; and
- a logout/login is needed only when group membership is newly added.

An inspect-first alternative downloads the installer before executing it:

```sh
curl -fLO https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh
less install.sh
bash install.sh
```

A separate integrity subsection shows how to download `install.sh` and
`SHA256SUMS` from one explicitly pinned release tag and verify only the
`install.sh` manifest entry before inspection. It must not download the two
files through separate `latest` lookups, which could straddle a newly
published release.

Manual package installation remains documented as a fallback. Source-build
documentation uses:

```sh
sudo ./packaging/install-from-source.sh
```

The documentation also explains that checksums and immutable GitHub releases
do not make a piped script equivalent to local review; users who prefer review
should use the inspect-first path.

## Failure handling

- Unsupported platform, architecture, root invocation, missing prerequisite,
  malformed manifest, failed checksum, wrong metadata, or downgrade:
  exit before sudo and leave no persistent files.
- Download interruption: delete partial files and the private temporary
  directory.
- Sudo refusal: exit without package or service changes.
- Package transaction failure: report the native package-manager error and do
  not continue to service/group success messages.
- Service failure after installation: report the installed-package state and
  bounded service diagnostics, then exit nonzero.
- Group reconciliation failure: leave the installed service intact, explain
  that the app cannot access `/run/fangd.sock`, and provide the exact manual
  recovery command for the captured user.
- Draft construction failure: never publish a partial release.
- Post-publication immutable assertion failure: treat it as a release
  incident, do not mutate the published release, and do not advertise the
  installer URL until investigated.

## Security and trust boundary

The one-line flow trusts:

- the `bladeandsoulx/fang-razer-linux` GitHub repository;
- GitHub's HTTPS delivery;
- the release workflow and repository credentials;
- the immutable tag and release assets; and
- the host's native package manager and sudo policy.

SHA-256 catches corrupted downloads and package/manifest mix-ups. It does not
provide an independent signature because the manifest is delivered beside the
packages. GitHub release immutability prevents later tag movement or asset
replacement for a published release and automatically creates a release
attestation covering the tag, commit, and assets.

No downloaded shell fragment other than the selected `install.sh` is
executed. Package metadata is inspected as data before installation. Filenames
and URLs come from release-embedded constants, not network responses or
untrusted platform strings.

## Success criteria

This work is complete when:

- a normal desktop user on the declared x86_64 platforms can run the exact
  one-line command and install both components;
- Zorin 18.1 selects the Ubuntu 24.04 DEB family automatically with the
  compatibility warning;
- validation and downgrade failures occur before any sudo prompt;
- fresh, upgrade, and equal/equal runs follow the decision table;
- the service is enabled and active and the captured user is reconciled into
  `fang`;
- the first installer-enabled release has exactly six manually attached
  assets and a five-entry checksum manifest;
- all DEB, RPM, installer, ShellCheck, checksum, metadata, and inventory gates
  pass before draft creation;
- the draft is validated and published exactly once;
- the resulting GitHub release reports `immutable: true`;
- the latest-release installer URL resolves successfully;
- the README documents both one-line and inspect-first flows;
- `packaging/install.sh` no longer collides with the release installer name;
  and
- `v0.9.3` remains unchanged.

## References

- [GitHub: Linking to releases](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)
- [GitHub: Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub: Preventing changes to releases](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes)
- [GitHub REST API: repository immutable releases](https://docs.github.com/en/rest/repos/repos#check-if-immutable-releases-are-enabled-for-a-repository)
- [GitHub REST API: releases](https://docs.github.com/en/rest/releases/releases)
- [RPM version and EVR semantics](https://rpm-software-management.github.io/rpm/man/rpm-version.7)
- [RPM Lua `vercmp`](https://rpm-software-management.github.io/rpm/man/rpm-lua.7)
- [Debian package version comparison](https://www.debian.org/doc/debian-policy/ch-controlfields.html#version)
