# Documentation Synchronization and DEB Rebuild Design

**Date:** 2026-07-23

**Status:** Approved

## Objective

Bring Fang's current documentation and in-app changelog into sync with the
v0.9.4 installer release and the accepted USDT networks, then produce a fresh,
matching Ubuntu/Debian package pair on the local Desktop workspace without
installing either package system-wide.

The required build outputs are:

- `Fang_0.9.4_amd64.deb`
- `fangd_0.9.4-1_amd64.deb`

Both packages will be collected in `target/deb-dist/`.

## Scope

### Current documentation

Audit and update the maintained, current-facing documentation:

- `README.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`
- `HARDWARE_TESTING.md`
- `app/src/screens/Changelog.svelte`

Only files that need a factual or consistency correction will be changed.
Historical plans and specifications under `docs/superpowers/` will remain
unchanged because they describe decisions and implementation state at the time
they were written.

### Support and changelog wording

The current Support screen remains the product source of truth:

- USDT is accepted on BNB Smart Chain (BEP20).
- USDT is accepted on Ethereum (ERC20).
- The previous transfer-warning paragraph is no longer shown.

The repository changelog will preserve the v0.9.2 historical record that the
warning was introduced in that release. The v0.9.4 entry will state that the
warning was removed, so the history remains accurate rather than being
rewritten.

The in-app changelog will be brought forward from v0.9.2 by adding condensed
v0.9.3 and v0.9.4 entries. Its v0.9.4 entry will cover:

- the immutable six-asset release installer;
- validation of the matching app and daemon packages before elevation;
- the two accepted USDT networks; and
- removal of the previous transfer-warning paragraph.

## Package Build

The rebuild will follow the same commands used by continuous integration:

1. Build the daemon package with `cargo deb -p fangd`.
2. Install frontend dependencies from the lockfile with `npm ci --prefix app`.
3. Build only the desktop DEB bundle with
   `npm --prefix app run tauri -- build --bundles deb`.
4. Replace the contents of `target/deb-dist/` with the exact v0.9.4 DEB pair.

The build is local to `/home/home/Desktop/fang-Fabel5`. It will not invoke the
source installer, install either DEB, enable `fangd`, change group membership,
or otherwise modify the operating system.

## Validation

Documentation and source validation will include:

- searching current user-facing sources for the removed warning text;
- checking that current USDT wording names BEP20 and ERC20;
- checking Markdown whitespace and balanced fenced code blocks;
- running the version consistency checker;
- running relevant app and installer tests; and
- confirming the production desktop build succeeds.

Package validation will be non-installing and will confirm:

- exactly two DEBs exist in `target/deb-dist/`;
- filenames match v0.9.4;
- package names are `fang` and `fangd`;
- versions are `0.9.4` and `0.9.4-1`;
- architecture is `amd64`;
- the desktop package depends on the compatible v0.9.4 daemon range; and
- package contents include the expected desktop binary, daemon binary,
  desktop entry, and systemd service.

The existing `packaging/deb/verify.sh` lifecycle verifier installs and removes
packages, so it will not be run on the host for this task. Equivalent
read-only metadata and archive-content checks will be used instead.

## Failure Handling

If a documentation or test check fails, fix the source and rerun the failing
check before continuing. If a package build fails because a required local
tool or system library is missing, report the exact dependency and request
permission before performing any system-level installation.

Old packages in their original build directories may remain as build-cache
artifacts. `target/deb-dist/` is the authoritative handoff directory and must
contain only the newly built v0.9.4 pair.

## Completion Criteria

The task is complete when:

- maintained current documentation is consistent with v0.9.4;
- the in-app changelog contains v0.9.3 and v0.9.4;
- the removed transfer-warning paragraph is absent from current UI copy;
- USDT consistently lists BEP20 and ERC20;
- relevant tests and production build checks pass;
- `target/deb-dist/` contains exactly the requested DEB pair; and
- the package metadata and contents pass the non-installing validation.
