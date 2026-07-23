# Fang Release Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Fang 0.9.4 with a release-locked one-command installer and a fail-closed, immutable six-asset release pipeline.

**Architecture:** A root `install.sh` performs platform detection, release-locked downloads, checksum and package-metadata validation, native version comparison, one native package transaction, and service/group reconciliation. Host-runnable Node tests exercise it through isolated command stubs, while a separate Node release-contract tool stages and validates the exact asset inventory. GitHub Actions builds and tests both package families before one API-driven draft is validated and published.

**Tech Stack:** Bash, Node.js 22 test runner, DEB tooling, RPM/DNF tooling, GitHub Actions, GitHub REST API, ShellCheck

## Global Constraints

- `v0.9.3` remains unchanged; the first installer-enabled release is `v0.9.4`.
- The release contains exactly `install.sh`, `SHA256SUMS`, two DEBs, and two RPMs.
- `SHA256SUMS` contains exactly five canonical entries and never hashes itself.
- Only exact `x86_64` hosts are accepted; DEBs are `amd64` and RPMs are `x86_64`.
- Direct platforms are Ubuntu 22.04/24.04, Debian 12/13, and Fedora 43/44.
- Supported derivatives require an explicit supported base marker and receive a warning.
- Root invocation, ambiguous platform data, missing prerequisites, bad checksums, wrong package metadata, and downgrades fail before `sudo`.
- Both selected packages are always submitted to one native transaction unless both are already equal.
- Equal/equal runs skip package installation but still reconcile the service and group.
- The installer contains only function definitions before one final `main "$@"` line.
- Publication creates one draft, uploads six explicit files, validates it, publishes once, and asserts immutability.
- The immutability preflight uses a scoped `IMMUTABLE_RELEASES_TOKEN` repository secret with Administration read permission.

---

### Task 1: Lock v0.9.4 release identity across manifests and tools

**Files:**
- Create: `packaging/release/release-contract.mjs`
- Create: `packaging/release/release-contract.test.mjs`
- Modify: `app/scripts/version.mjs`
- Modify: `app/scripts/version.test.mjs`
- Modify: `CHANGELOG.md`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `app/package.json`
- Modify: `app/package-lock.json`
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/Cargo.lock`
- Modify: `app/src-tauri/tauri.conf.json`
- Modify: `packaging/rpm/fang.spec`
- Modify: `packaging/rpm/fangd.spec`
- Modify: `docs/superpowers/specs/2026-07-18-release-installer-design.md`

**Interfaces:**
- Consumes: repository manifests and four package artifact paths.
- Produces: `releaseNames(version)`, `checksumNames(version)`, `stageRelease({version, debDir, rpmDir, outputDir, installer})`, and `validateManifest(text, expectedNames)`.

- [ ] **Step 1: Write failing release-contract tests**

```javascript
test('0.9.4 owns six exact assets and five checksum entries', () => {
  assert.deepEqual(releaseNames('0.9.4'), [
    'install.sh',
    'SHA256SUMS',
    'Fang_0.9.4_amd64.deb',
    'fangd_0.9.4-1_amd64.deb',
    'fang-0.9.4-1.x86_64.rpm',
    'fangd-0.9.4-1.x86_64.rpm'
  ]);
  assert.deepEqual(checksumNames('0.9.4'), releaseNames('0.9.4').filter((name) => name !== 'SHA256SUMS'));
});

test('manifest rejects missing, duplicate, malformed, path, and extra entries', () => {
  const expected = checksumNames('0.9.4');
  const valid = expected.map((name) => `${'a'.repeat(64)}  ${name}\n`).join('');
  assert.doesNotThrow(() => validateManifest(valid, expected));
  for (const malformed of [
    valid.replace(/^.*\n/, ''),
    valid + valid.split('\n')[0] + '\n',
    valid.replace(/[a-f0-9]{64}/, 'BAD'),
    valid.replace('install.sh', '../install.sh'),
    valid + `${'b'.repeat(64)}  seventh.asset\n`
  ]) {
    assert.throws(() => validateManifest(malformed, expected));
  }
});
```

- [ ] **Step 2: Run the tests and confirm missing-module failure**

Run: `node --test packaging/release/release-contract.test.mjs`

Expected: FAIL because `release-contract.mjs` does not exist.

- [ ] **Step 3: Implement exact filename and manifest helpers**

```javascript
export function releaseNames(version) {
  return [
    'install.sh',
    'SHA256SUMS',
    `Fang_${version}_amd64.deb`,
    `fangd_${version}-1_amd64.deb`,
    `fang-${version}-1.x86_64.rpm`,
    `fangd-${version}-1.x86_64.rpm`
  ];
}

export function checksumNames(version) {
  return releaseNames(version).filter((name) => name !== 'SHA256SUMS');
}

export function validateManifest(text, expectedNames) {
  if (!text.endsWith('\n')) throw new Error('checksum manifest needs one final newline');
  const names = text.trimEnd().split('\n').map((line) => {
    const match = line.match(/^([a-f0-9]{64})  ([^/]+)$/);
    if (!match) throw new Error(`malformed checksum line: ${line}`);
    return match[2];
  });
  assertExactNames(names, expectedNames);
}
```

`stageRelease` copies only exact basenames into an empty output directory, verifies DEB/RPM metadata with `dpkg-deb` and `rpm`, writes the five hashes in `checksumNames` order, and calls `validateManifest` on the result.

- [ ] **Step 4: Extend version synchronization tests**

Add `install.sh` and `packaging/release/release-contract.mjs` to the isolated version fixture. Assert `set 0.10.0` updates `VERSION='0.10.0'`, `RELEASE_TAG='v0.10.0'`, and release filename formulas, while `check` rejects a stale installer version or tag.

- [ ] **Step 5: Extend `version.mjs` and bump the repository**

Teach `currentVersions()` and `setVersion()` to read and replace the installer `VERSION`/`RELEASE_TAG`. Run:

```bash
node app/scripts/version.mjs set 0.9.4
```

Add the 0.9.4 changelog entry and link, and change the installer design status to `Approved`.

- [ ] **Step 6: Run focused verification**

Run:

```bash
node --test app/scripts/version.test.mjs packaging/release/release-contract.test.mjs
node app/scripts/version.mjs check
```

Expected: all tests pass and version sync reports `0.9.4`.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md app packaging/rpm docs/superpowers/specs/2026-07-18-release-installer-design.md packaging/release
git commit -m "build: prepare v0.9.4 installer release"
```

---

### Task 2: Implement safe platform parsing and release-locked downloads

**Files:**
- Create: `install.sh`
- Create: `packaging/installer/installer.test.mjs`
- Create: `packaging/installer/banner.txt`

**Interfaces:**
- Consumes: `/etc/os-release` or `FANG_OS_RELEASE_FILE` in tests, exact embedded release constants, and standard host commands.
- Produces: `parse_os_release`, `detect_platform`, `capture_identity`, `require_commands`, `download_file`, `parse_manifest`, and the final `main "$@"`.

- [ ] **Step 1: Build an isolated installer harness**

The test harness creates a mode-0700 temporary directory, a stub `PATH`, fixture os-release file, command log, package metadata map, installed-version map, and deterministic download assets. It invokes the real `install.sh` with:

```javascript
const result = spawnSync('bash', [installer], {
  env: {
    ...minimalEnv,
    PATH: stubPath,
    FANG_OS_RELEASE_FILE: osRelease,
    FANG_TEST_EUID: '1000',
    FANG_TEST_TTY: '0',
    FANG_TEST_COMMAND_LOG: log
  },
  encoding: 'utf8'
});
```

Tests must reject root, non-`x86_64`, duplicate/malformed os-release keys, unsupported direct releases, conflicting derivative markers, and unknown distributions without a logged `sudo`.

- [ ] **Step 2: Add direct and derivative detection cases**

Use table-driven cases for Ubuntu 22.04/24.04, Debian 12/13, Fedora 43/44, Zorin/Mint/Pop!_OS on supported Ubuntu bases, a Debian derivative with `bookworm`/`trixie`, and a Fedora derivative with `platform:f43`/`platform:f44`. Assert family, exact asset names, and whether the compatibility warning appears.

- [ ] **Step 3: Run the focused tests and confirm failure**

Run: `node --test --test-name-pattern='platform|identity|root|architecture' packaging/installer/installer.test.mjs`

Expected: FAIL because `install.sh` does not exist.

- [ ] **Step 4: Implement strict os-release parsing**

`parse_os_release` reads only `ID`, `ID_LIKE`, `VERSION_ID`, `VERSION_CODENAME`, `UBUNTU_CODENAME`, and `PLATFORM_ID`; rejects duplicate relevant keys; decodes single/double-quoted values without `source` or `eval`; rejects command substitution, control characters, malformed quoting, and characters outside each field allowlist.

`detect_platform` applies the seven ordered rules from the design and sets `PACKAGE_FAMILY`, `PLATFORM_LABEL`, and `DERIVATIVE_WARNING`.

- [ ] **Step 5: Implement streaming-safe setup and downloads**

All constants are assigned inside `main`. `download_file` uses:

```bash
curl --fail --show-error --silent --location \
  --proto '=https' --proto-redir '=https' \
  --retry 3 --retry-delay 1 \
  --output "$partial" "$url"
mv -- "$partial" "$destination"
```

The script creates a private `mktemp -d`, registers cleanup for `EXIT HUP INT TERM`, downloads the pinned `SHA256SUMS` and only the chosen package pair, and has no executable top-level statement except final `main "$@"`.

- [ ] **Step 6: Add truncation and cleanup tests**

Generate every line-boundary prefix of `install.sh`; each prefix that parses and lacks the final `main "$@"` must produce an empty command log. Add success, download-failure, and signal cases that assert removal of the temporary directory and `.part` files.

- [ ] **Step 7: Run focused verification**

Run: `node --test --test-name-pattern='platform|identity|root|architecture|download|truncation|cleanup' packaging/installer/installer.test.mjs`

Expected: all focused cases pass.

- [ ] **Step 8: Commit**

```bash
git add install.sh packaging/installer
git commit -m "feat(installer): detect supported release platforms"
```

---

### Task 3: Validate checksums, package metadata, and installed versions

**Files:**
- Modify: `install.sh`
- Modify: `packaging/installer/installer.test.mjs`

**Interfaces:**
- Consumes: downloaded manifest/package pair and installed package databases.
- Produces: `verify_checksums`, `verify_deb_metadata`, `verify_rpm_metadata`, `deb_state`, `rpm_state`, and `decide_transaction`.

- [ ] **Step 1: Add failing checksum and metadata tests**

Cover missing/extra/duplicate/malformed manifest lines, path components, wrong hashes for each selected package, and every wrong DEB/RPM field: name, complete version or EVR, epoch, release, and architecture. Every failure must leave the `sudo` log empty.

- [ ] **Step 2: Add failing installed-version matrix tests**

For each package family, test absent/older/equal/newer app and daemon combinations. Assert newer/newer-or-mixed refuses before `sudo`, equal/equal skips the package transaction, and every accepted mixed state sends both absolute package paths.

Include `1:0.9.3-1`, Debian revisions, and RPM epoch cases so lexical comparison or `sort -V` would fail.

- [ ] **Step 3: Run focused tests and confirm failure**

Run: `node --test --test-name-pattern='checksum|metadata|version|downgrade|transaction' packaging/installer/installer.test.mjs`

Expected: FAIL at the first unimplemented validation phase.

- [ ] **Step 4: Implement exact manifest and metadata validation**

Parse exactly five canonical manifest entries, compare the name set, write a two-line selected manifest in the temporary directory, and run `sha256sum -c`. Query every DEB field independently with `dpkg-deb -f`, and every RPM field independently with one fixed `rpm -qp --queryformat`.

- [ ] **Step 5: Implement native version classification**

DEB uses `dpkg-query` plus `dpkg --compare-versions`. RPM rejects multiple installed records and compares EVRs through fixed RPM Lua loaded from a file/data argument rather than interpolating package values as code.

- [ ] **Step 6: Implement the decision table**

Any newer package prints its name and both versions and returns before `sudo`. Equal/equal sets `PACKAGE_TRANSACTION=0`; every other accepted state sets it to `1` and invokes one `apt-get install` or `dnf install` with both absolute package paths.

- [ ] **Step 7: Run focused and complete installer tests**

Run:

```bash
node --test --test-name-pattern='checksum|metadata|version|downgrade|transaction' packaging/installer/installer.test.mjs
node --test packaging/installer/installer.test.mjs
```

Expected: all cases pass.

- [ ] **Step 8: Commit**

```bash
git add install.sh packaging/installer/installer.test.mjs
git commit -m "feat(installer): verify packages before elevation"
```

---

### Task 4: Reconcile service/group state and terminal behavior

**Files:**
- Modify: `install.sh`
- Modify: `packaging/installer/installer.test.mjs`
- Modify: `packaging/installer/banner.txt`

**Interfaces:**
- Consumes: validated selected packages, captured user, and transaction decision.
- Produces: `install_packages`, `reconcile_service`, `reconcile_group`, `print_banner`, `step`, `warn`, and `fatal`.

- [ ] **Step 1: Add failing mutation and output tests**

Assert validation completes before `sudo -v`; one package transaction receives both paths; equal/equal performs no package install; inactive service is enabled; service failure runs bounded `systemctl status --no-pager --lines=20`; missing group is fatal; existing membership skips `usermod`; new membership adds only the captured user and prints logout guidance.

Snapshot the non-color banner and assert TTY color is present only when `NO_COLOR` is unset.

- [ ] **Step 2: Run focused tests and confirm failure**

Run: `node --test --test-name-pattern='sudo|service|group|banner|color|idempotent' packaging/installer/installer.test.mjs`

Expected: FAIL because reconciliation/output functions are missing.

- [ ] **Step 3: Implement one elevated mutation phase**

Call `sudo -v` only after all validation. Install with one native transaction when required, confirm `getent group fang`, run `sudo systemctl enable --now fangd`, verify `systemctl is-active --quiet fangd`, and conditionally run `sudo usermod -aG fang "$TARGET_USER"`.

- [ ] **Step 4: Implement stable terminal output**

Match `packaging/installer/banner.txt` byte-for-byte without color. Keep raw package-manager output visible, use the four specified symbols consistently, and print manual recovery commands containing only the already-validated target username.

- [ ] **Step 5: Run the complete installer suite**

Run:

```bash
bash -n install.sh
node --test packaging/installer/installer.test.mjs
```

Expected: syntax and all fixture tests pass.

- [ ] **Step 6: Commit**

```bash
git add install.sh packaging/installer
git commit -m "feat(installer): install and reconcile Fang"
```

---

### Task 5: Add DEB lifecycle gates and CI validation

**Files:**
- Create: `packaging/deb/verify.sh`
- Create: `packaging/deb/verify.test.mjs`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: exact DEB pair built on Ubuntu 22.04.
- Produces: a reusable verifier for Ubuntu 22.04/24.04 and Debian 12/13 containers.

- [ ] **Step 1: Add static verifier/workflow contract tests**

Assert the verifier checks exact filenames and metadata, installs both absolute paths in one `apt-get` command, verifies group/unit/mock daemon/desktop libraries/startup/package integrity/removal, and the workflows use all four declared container images.

- [ ] **Step 2: Run the tests and confirm failure**

Run: `node --test packaging/deb/verify.test.mjs`

Expected: FAIL because the verifier does not exist.

- [ ] **Step 3: Implement the DEB verifier**

Mirror the existing RPM verifier while using `dpkg-deb`, `apt-get`, `dpkg-query`, `dpkg -V`, `systemd-analyze verify`, the mock socket smoke test, `ldd`, `dbus-run-session`, Xvfb, and package removal/file ownership checks.

- [ ] **Step 4: Add CI and release matrices**

The build job uploads the exact DEBs once. A four-image matrix downloads the same artifact and runs `packaging/deb/verify.sh`. Add installer, release-contract, source-version, ShellCheck, frontend, Rust, and existing RPM checks as independent prerequisites.

- [ ] **Step 5: Run host-runnable checks**

Run:

```bash
node --test packaging/deb/verify.test.mjs
node --test packaging/rpm/*.test.mjs packaging/release/*.test.mjs packaging/installer/*.test.mjs
```

Expected: all host contract tests pass.

- [ ] **Step 6: Commit**

```bash
git add packaging/deb .github/workflows
git commit -m "ci: test Fang DEBs across supported bases"
```

---

### Task 6: Publish one validated immutable release

**Files:**
- Create: `packaging/release/publish.sh`
- Create: `packaging/release/publish.test.mjs`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: a staged six-file directory, tag, `GITHUB_REPOSITORY`, `GITHUB_SHA`, `GITHUB_TOKEN`, and `IMMUTABLE_RELEASES_TOKEN`.
- Produces: one published latest immutable release or a retained unpublished draft on failure.

- [ ] **Step 1: Add failing publication contract tests**

Assert a per-tag concurrency group, immutable preflight, existing-release refusal, no globs/clobber flags, one draft creation, six explicit uploads, draft asset name/size/digest comparison, one publish request with `make_latest= true`, and postconditions for tag/draft/prerelease/immutable/latest.

- [ ] **Step 2: Run the tests and confirm failure**

Run: `node --test packaging/release/publish.test.mjs`

Expected: FAIL because `publish.sh` does not exist.

- [ ] **Step 3: Implement fail-closed API publication**

Use `gh api` with `X-GitHub-Api-Version: 2026-03-10`. Query `/immutable-releases` using `IMMUTABLE_RELEASES_TOKEN`; use the normal contents-write token for release creation/upload/update. Never delete or mutate a pre-existing release. Leave a newly-created draft intact when later validation fails.

- [ ] **Step 4: Validate remote asset metadata**

Compare exact names, count, sizes, and `sha256:` digests from the release API with local files before publication. Publish with one PATCH setting `draft=false`, `prerelease=false`, and `make_latest=true`, then re-read the release and latest endpoints.

- [ ] **Step 5: Replace the old draft-release workflow job**

Stage via `release-contract.mjs`, upload only six explicit paths, run `publish.sh`, and require DEB/RPM matrices, installer fixtures, ShellCheck, version sync, and release contracts before the publication job.

- [ ] **Step 6: Run publication contract tests**

Run:

```bash
bash -n packaging/release/publish.sh
node --test packaging/release/publish.test.mjs
```

Expected: all cases pass without network access.

- [ ] **Step 7: Commit**

```bash
git add packaging/release .github/workflows/release.yml
git commit -m "ci(release): publish immutable six-asset releases"
```

---

### Task 7: Rename the source installer and document both install paths

**Files:**
- Move: `packaging/install.sh` to `packaging/install-from-source.sh`
- Modify: `README.md`
- Modify: `HARDWARE_TESTING.md`
- Modify: `CONTRIBUTING.md`
- Modify: `docs/superpowers/specs/2026-07-18-release-installer-design.md`

**Interfaces:**
- Consumes: root release installer and renamed source builder.
- Produces: one-line, inspect-first, pinned-integrity, manual-package, and source-build documentation.

- [ ] **Step 1: Add documentation/source-name contract tests**

Extend release-contract tests to assert `packaging/install.sh` is absent, `install-from-source.sh` is executable, README contains the exact one-line and inspect-first commands, pinned-tag checksum instructions, supported platform matrix, non-root warning, downgrade policy, and immutable-release token prerequisite.

- [ ] **Step 2: Run the test and confirm failure**

Run: `node --test --test-name-pattern='documentation|source installer' packaging/release/release-contract.test.mjs`

Expected: FAIL while the old filename and README remain.

- [ ] **Step 3: Move the source-build script**

Use `git mv packaging/install.sh packaging/install-from-source.sh`, update its usage comment, and preserve executable mode.

- [ ] **Step 4: Rewrite installation documentation**

Lead with the one-line command, then non-root/pre-sudo guarantees, supported bases and derivatives, inspect-first flow, pinned `v0.9.4` integrity flow, manual packages, and renamed source build. Document the required immutable-release repository setting and scoped secret for maintainers.

- [ ] **Step 5: Run documentation and shell checks**

Run:

```bash
node --test packaging/release/release-contract.test.mjs
shellcheck install.sh packaging/install-from-source.sh packaging/rpm/build.sh packaging/rpm/verify.sh packaging/deb/verify.sh packaging/release/publish.sh
```

Expected: all tests and ShellCheck pass.

- [ ] **Step 6: Commit**

```bash
git add README.md HARDWARE_TESTING.md CONTRIBUTING.md packaging docs/superpowers/specs/2026-07-18-release-installer-design.md
git commit -m "docs: publish the Fang installer workflow"
```

---

### Task 8: Full verification and release-readiness review

**Files:**
- Modify only files required by failures discovered in this task.

**Interfaces:**
- Consumes: completed implementation.
- Produces: fresh evidence for code, frontend, packaging, installer, and workflow contracts.

- [ ] **Step 1: Run formatting, unit, build, and package-contract checks**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm test --prefix app
npm run build --prefix app
cargo fmt --check --manifest-path app/src-tauri/Cargo.toml
cargo clippy --manifest-path app/src-tauri/Cargo.toml --bin fang --all-targets -- -D warnings
cargo test --manifest-path app/src-tauri/Cargo.toml --bin fang
node app/scripts/version.mjs check
node --test app/scripts/version.test.mjs app/src/lib/*.test.js packaging/**/*.test.mjs
python3 packaging/rpm/mock_smoke_test.py
shellcheck install.sh packaging/install-from-source.sh packaging/rpm/build.sh packaging/rpm/verify.sh packaging/deb/verify.sh packaging/release/publish.sh
```

Expected: every command exits zero.

- [ ] **Step 2: Review repository invariants**

Run:

```bash
git diff --check
git status --short
rg -n 'packaging/install\.sh|0\.9\.3|four-package draft' README.md HARDWARE_TESTING.md CONTRIBUTING.md .github packaging app/scripts
```

Expected: no whitespace errors, only intended changes, and no stale installer/release references outside historical text.

- [ ] **Step 3: Commit verification fixes, if any**

```bash
git add -A
git commit -m "test: close v0.9.4 release gates"
```

