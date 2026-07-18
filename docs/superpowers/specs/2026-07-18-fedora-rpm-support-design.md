# Fedora RPM support for Fang

**Date:** 2026-07-18
**Status:** Draft — approved in conversation; awaiting written-spec review

## Goal

Ship installable, release-quality RPM packages for Fang on Fedora without
requiring a Fedora installation on the maintainer's computer. Every GitHub
release created by the updated workflow will contain two x86_64 RPMs:

1. `fangd`, the privileged daemon and its systemd integration.
2. `fang`, the Tauri desktop application.

The packages target Fedora 43 and Fedora 44. GitHub Actions builds them on
Fedora 43, the oldest supported Fedora release, and validates the same binaries
on both Fedora versions before a release can be created.

## Scope

### In scope

- Fedora 43 and Fedora 44 on x86_64.
- A custom RPM spec for `fangd`.
- Tauri's native RPM bundler for the desktop application.
- Fedora-native systemd and sysusers integration.
- RPM build and install tests in GitHub Actions containers.
- Two RPM assets alongside the existing two DEB assets in each GitHub release.
- Fedora installation and hardware-validation documentation.
- A `fangd --version` command suitable for package smoke tests.

### Out of scope

- RHEL, CentOS Stream, Rocky Linux, AlmaLinux, or other RPM distributions.
- Fedora 42 or older.
- aarch64 and other architectures.
- COPR, DNF repository metadata, automatic repository updates, or Fedora
  official-repository submission.
- RPM signing in this first iteration.
- Source RPMs as release assets.
- A Fedora from-source installer.
- Changing the existing Debian packages' install, enable, or start behavior.
- Claiming real-hardware, enforcing-SELinux, or full desktop-session validation
  from container-only CI.

## Package design

### `fangd` daemon RPM

Add a focused RPM packaging directory:

```text
packaging/rpm/
├── fangd.spec
└── fang.sysusers
```

The release workflow compiles `fangd` in a Fedora 43 container, and
`fangd.spec` packages that release binary. The binary is installed as
`/usr/bin/fangd`. The package also installs:

- the existing `packaging/fangd.service` as
  `%{_unitdir}/fangd.service`;
- `packaging/rpm/fang.sysusers` as
  `%{_sysusersdir}/fang.conf`; and
- the repository's `LICENSE` as an RPM `%license` file.

The sysusers declaration creates the system group `fang`, which owns access to
`/run/fangd.sock`. Packaging must use Fedora's systemd/sysusers RPM macros
rather than a distribution-specific `groupadd` script. The spec declares the
macro build requirements and uses Fedora's standard service lifecycle macros
for install, upgrade, and removal:

- `%systemd_post fangd.service`
- `%systemd_preun fangd.service`
- `%systemd_postun_with_restart fangd.service`

Installing the RPM must not unconditionally start or enable the daemon from a
custom scriptlet. Fedora's preset policy remains authoritative, and Fang's
documented deterministic setup command is:

```sh
sudo systemctl enable --now fangd
```

The daemon RPM is versioned from the workspace version. Its RPM `Release`
starts at `1`, so a package-only correction can increment the release without
changing Fang's application version. Its architecture must be `x86_64`, and
automatic ELF dependency generation remains enabled.

`fangd --version` prints `fangd <version>` and exits successfully without
opening hardware, creating state, or binding a socket. Unknown options continue
to exit with status 2.

### `fang` desktop RPM

Extend `app/src-tauri/tauri.conf.json` so Tauri builds both `deb` and `rpm`
bundles. The RPM configuration declares a strict daemon dependency for the
same compatible release line:

```text
fangd >= <current version>
fangd < <next minor version>
```

For example, desktop version `0.9.2` accepts `fangd >= 0.9.2` and
`fangd < 0.10.0`. This mirrors the existing DEB policy while using RPM
dependency syntax. The desktop package remains unprivileged and contains no
daemon binary, service unit, group-creation script, or elevated install hook.

The installed desktop package name is `fang`. The release asset may retain the
Tauri-generated filename casing, but CI identifies packages from RPM metadata,
not from filename capitalization.

The desktop RPM includes the project's GPL-2.0 license metadata and relies on
Tauri/RPM's generated shared-library dependencies for GTK, WebKitGTK 4.1,
Ayatana AppIndicator, and the other native runtime libraries.

### Version synchronization

Extend `app/scripts/version.mjs` so `check` verifies both DEB and RPM daemon
constraints, and `set VERSION` updates both formats together. The next-minor
upper bound is calculated once from the application version. A version bump
must fail CI if any Cargo, npm, Tauri, changelog, DEB dependency, or RPM
dependency version is out of sync.

## CI design

No Fedora host installation is required. GitHub-hosted Ubuntu runners execute
the Fedora jobs inside official `fedora:43` and `fedora:44` containers.

Keep packaging behavior in small checked-in scripts under `packaging/rpm/`
where practical, so pull-request and tag workflows run the same build and
validation commands rather than maintaining two independent implementations.

### Build job

Every pull request and push to `main` adds an RPM build job using
`container: fedora:43`. It installs the Fedora build dependencies, Node 22, and
the Rust stable toolchain, then:

1. Runs the repository's version-sync check.
2. Builds the `fangd` release binary.
3. Builds the daemon RPM with `rpmbuild`.
4. Runs `npm ci` and builds the desktop with
   `npm run tauri build -- --bundles rpm`.
5. Verifies that exactly one daemon RPM and one desktop RPM were produced.
6. Uploads both binary RPMs as one workflow artifact for install tests.

Fedora 43 is the build baseline because binaries built against a newer glibc
may not run on an older supported release. The same Fedora 43-built artifacts
are tested on Fedora 43 and 44; Fedora 44 does not rebuild them.

### Fedora 43/44 install-test matrix

Two matrix jobs download the build artifact and run in clean `fedora:43` and
`fedora:44` containers. Each job must verify:

1. **Static RPM metadata**
   - exactly two binary RPMs exist;
   - package names are `fangd` and `fang`;
   - version, release, architecture, license, and package ownership are
     correct;
   - `fangd` contains `/usr/bin/fangd`, `fangd.service`, the sysusers file,
     and the license;
   - `fang` contains the desktop binary, desktop entry, icons, and license
     metadata;
   - the desktop package requires the expected lower and upper `fangd`
     bounds; and
   - a deliberately incompatible daemon version cannot satisfy those bounds.

2. **Clean DNF installation**
   - `dnf install` resolves runtime dependencies and installs both local RPMs;
   - the installed versions and architecture match the build;
   - `getent group fang` succeeds after installation; and
   - `rpm -V fangd fang` reports no altered or missing packaged files.

3. **Daemon smoke tests**
   - `/usr/bin/fangd --version` prints the package version and exits zero;
   - the installed unit passes `systemd-analyze verify`; and
   - the installed daemon starts in mock mode on a loopback endpoint, answers
     a public JSON-lines `get_status` request with `mock: true`, and exits
     cleanly when the test stops it, without accessing Razer hardware.

4. **Desktop smoke tests**
   - `ldd` reports no unresolved shared libraries for `/usr/bin/fang`; and
   - under `dbus-run-session` and Xvfb, the installed app starts and stays
     alive for a bounded smoke window without a loader or immediate startup
     failure. The test then terminates the process itself.

5. **Removal**
   - DNF removes both packages successfully; and
   - files owned by the two packages are gone. State created at runtime and
     the shared system group are not treated as package-file leaks.

Containers do not run systemd as PID 1. CI therefore validates the unit
statically and exercises `fangd` directly in mock mode; it does not claim that
`systemctl start fangd` ran inside the container.

## Release workflow

Refactor `.github/workflows/release.yml` into gated artifact jobs:

```text
debs ───────────────────────────────┐
                                    ├─> draft-release
rpms ──> rpm-tests (43 and 44) ─────┘
```

- `debs` preserves the current Ubuntu 22.04 build and uploads the daemon and
  desktop DEBs as a workflow artifact.
- `rpms` builds both RPMs in Fedora 43 and uploads them as a workflow artifact.
- `rpm-tests` downloads those exact RPMs and runs the Fedora 43/44 install
  matrix described above.
- `draft-release` runs only after the DEB build and both RPM test jobs succeed.
  It downloads, inventories, and attaches exactly four binary packages to one
  draft GitHub release: two DEBs and two RPMs.

The draft release continues to use generated release notes and the pushed
`v*` tag. No job may create or update a GitHub release before all prerequisite
jobs have succeeded. A missing, duplicate, or unexpected package fails the
inventory step rather than publishing a partial release.

Pull-request CI and tag-release CI call the same RPM scripts. Repository rules
should require the normal CI workflow on `main`; the tag workflow independently
reruns the Fedora package tests so a directly pushed tag still cannot create a
draft with untested RPMs.

## User documentation

Add a Fedora 43/44 section next to the existing Ubuntu/Debian instructions.
Users download both RPMs from the same GitHub release and run:

```sh
sudo dnf install ./fangd-*.rpm ./Fang-*.rpm
sudo systemctl enable --now fangd
sudo usermod -aG fang "$USER"
```

If Tauri emits a lowercase desktop filename, the documentation uses the actual
release filename instead of assuming `Fang-*.rpm`. Users must log out and back
in after `usermod` before the desktop app can access `/run/fangd.sock`.

The documentation also states:

- supported versions are Fedora 43 and 44, x86_64;
- the RPMs are direct GitHub downloads, not a configured DNF repository;
- the first RPM iteration is unsigned;
- uninstall commands are `sudo dnf remove fang fangd`;
- daemon diagnostics use `systemctl status fangd` and
  `journalctl -u fangd`; and
- Fedora users validating real Razer hardware should follow
  `HARDWARE_TESTING.md` and report their Fedora version, desktop session,
  SELinux status/denials, laptop model, and USB PID.

## Failure handling

- Build or validation failure in either package family prevents draft-release
  creation.
- Package discovery uses RPM metadata and asserts exact counts, avoiding
  filename-case assumptions and stale-artifact uploads.
- All install-test commands run with strict shell error handling.
- Mock daemon and desktop smoke tests have bounded timeouts and always clean up
  child processes.
- Test logs include RPM metadata, dependency output, unit verification output,
  and relevant application stderr so failures can be diagnosed without a
  Fedora workstation.
- The workflow never uploads an RPM built on Fedora 44 as a substitute for a
  failed Fedora 43 baseline build.

## Validation boundary

Container CI gives strong evidence that the RPMs are structurally correct,
dependency-complete, installable, launchable, and removable on Fedora 43 and
44. It cannot establish all behavior of a real Fedora laptop:

- no physical Razer HID/EC commands are sent;
- a complete GNOME or KDE login session, tray, autostart, and Wayland behavior
  are not exercised;
- systemd is not PID 1 in the test container;
- SELinux policy is not exercised in enforcing mode against real devices; and
- NVIDIA, DDC/CI, suspend/resume, and real sensor behavior remain untested.

The release documentation must describe Fedora support as package-tested until
the existing hardware checklist has been completed by maintainers or community
testers on real Fedora systems. Any SELinux denial or hardware-specific issue
found there is handled as a follow-up fix, not hidden by weakening CI.

## Success criteria

This work is complete when:

- CI builds two Fedora 43 x86_64 RPMs without a local Fedora installation.
- The exact RPMs install and pass the defined checks on Fedora 43 and 44.
- The desktop RPM enforces the compatible `fangd` release line.
- The daemon RPM installs the service and `fang` group integration using
  Fedora-native mechanisms.
- A tag cannot create a draft release unless both DEBs and both RPMs build and
  all Fedora package tests pass.
- The draft release contains exactly four binary package assets.
- Fedora installation, activation, group membership, uninstall, diagnostics,
  and validation limitations are documented.
- Existing Ubuntu/Debian packaging behavior and CI remain operational.

## References

- [Tauri RPM distribution guide](https://v2.tauri.app/distribute/rpm/)
- [GitHub Actions container jobs](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/run-jobs-in-a-container)
- [Fedora Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/)
- [Fedora WebKitGTK 4.1 packages](https://packages.fedoraproject.org/pkgs/webkitgtk/webkit2gtk4.1-devel/)
- [Fedora 44 release schedule](https://fedorapeople.org/groups/schedule/f-44/f-44-key-tasks.html)
