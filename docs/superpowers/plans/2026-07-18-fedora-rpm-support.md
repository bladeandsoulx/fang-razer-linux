# Fedora RPM Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build, test, publish, and document native `fangd` and `fang` RPMs for Fedora 43/44 without installing Fedora on the maintainer's computer.

**Architecture:** Fedora 43 GitHub Actions containers compile both Rust binaries. Two custom RPM specs package the daemon and desktop app, while one checked-in verifier installs the exact Fedora 43-built artifacts on Fedora 43 and 44. The tag workflow creates a draft only after both DEBs and both RPMs pass, then the verified draft is published as v0.9.3.

**Tech Stack:** Rust/Cargo, Tauri 2.11.4, Node 22, RPM/rpmbuild, systemd sysusers, Bash, Python 3, GitHub Actions, Fedora 43/44 containers.

## Global Constraints

- Support Fedora 43 and Fedora 44 on x86_64 only.
- Build both RPMs on Fedora 43; test those exact artifacts on Fedora 43 and Fedora 44.
- Produce exactly `fangd-0.9.3-1.x86_64.rpm` and `fang-0.9.3-1.x86_64.rpm` for the first Fedora-enabled release.
- Use custom `fangd.spec` and `fang.spec`; never invoke Tauri's RPM bundler.
- Tauri builds the desktop binary with `npm run tauri build -- --no-bundle`.
- At v0.9.3, `fang` requires `fangd >= 0.9.3` and `fangd < 0.10.0`.
- Keep automatic ELF dependency generation enabled and explicitly require `libayatana-appindicator-gtk3`.
- Ship the repository `LICENSE` as a `%license` payload in both RPMs.
- Ship `fang.sysusers`; rely on Fedora's native RPM sysusers handling with no custom `%pre` and no `%sysusers_create_compat`.
- Preserve all existing DEB build, install, enable, and start behavior.
- Release RPMs unsigned and directly through GitHub Releases; do not add COPR or repository metadata.
- Treat container CI as package validation, not real Razer hardware, SELinux-enforcing, or full desktop-session certification.
- Do not move or recreate the already-published `v0.9.2` tag. Fedora support first ships as patch release `v0.9.3`, keeping tag-to-source integrity.

## File Structure

### Create

- `packaging/rpm/fangd.spec` — daemon RPM metadata, file payload, and systemd lifecycle.
- `packaging/rpm/fang.spec` — desktop RPM metadata, strict daemon bounds, icons, desktop entry, and explicit tray dependency.
- `packaging/rpm/fang.sysusers` — declarative `fang` system group.
- `packaging/rpm/fang.desktop` — checked-in freedesktop launcher.
- `packaging/rpm/metadata.test.mjs` — host-runnable static packaging tests.
- `packaging/rpm/build.sh` — Fedora 43 build/staging entry point that emits exactly two RPMs.
- `packaging/rpm/build-script.test.mjs` — host-runnable build-script contract test.
- `packaging/rpm/mock_smoke.py` — installed-daemon JSON-lines smoke test.
- `packaging/rpm/mock_smoke_test.py` — unit tests for smoke-response parsing.
- `packaging/rpm/verify.sh` — clean-container metadata, dependency, install, launch, and removal checks.
- `app/scripts/version.test.mjs` — isolated version-sync CLI tests.

### Modify

- `crates/fangd/src/main.rs` — add side-effect-free `--version`/`-V`.
- `crates/fangd/tests/integration.rs` — verify version output and absence of state/socket side effects.
- `app/scripts/version.mjs` — synchronize both RPM specs and `fangd_upper`.
- `.github/workflows/ci.yml` — build RPMs on Fedora 43 and test them on Fedora 43/44.
- `.github/workflows/release.yml` — gate a four-package draft release on DEB/RPM builds and RPM matrix tests.
- `README.md` — Fedora install, activation, group, diagnostics, and removal instructions.
- `HARDWARE_TESTING.md` — Fedora/SELinux hardware-validation fields and DNF rollback.
- `.github/ISSUE_TEMPLATE/bug_report.yml` — Fedora-capable system example.
- `.github/ISSUE_TEMPLATE/model-support.yml` — Fedora version, session, and SELinux result fields.
- `Cargo.toml`, `Cargo.lock`, `app/package.json`, `app/package-lock.json`, `app/src-tauri/Cargo.toml`, `app/src-tauri/Cargo.lock`, `app/src-tauri/tauri.conf.json`, `CHANGELOG.md`, and both RPM specs — release v0.9.3 synchronization.

---

### Task 1: Add a side-effect-free daemon version command

**Files:**
- Modify: `crates/fangd/tests/integration.rs`
- Modify: `crates/fangd/src/main.rs`

**Interfaces:**
- Consumes: `env!("CARGO_PKG_VERSION")`.
- Produces: `fangd --version` and `fangd -V`, each writing `fangd 0.9.2\n` before the release bump and exiting zero without hardware, state, lock, or socket access.

- [ ] **Step 1: Write the failing integration test**

Add after `roundtrip` in `crates/fangd/tests/integration.rs`:

```rust
#[test]
fn version_flag_exits_without_creating_runtime_state() {
    let bin = env!("CARGO_BIN_EXE_fangd");
    let dir = std::env::temp_dir().join(format!("fangd-version-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let state = dir.join("state.json");
    let socket = dir.join("fangd.sock");

    let output = Command::new(bin)
        .args(["--mock", "--state"])
        .arg(&state)
        .args(["--socket"])
        .arg(&socket)
        .arg("--version")
        .output()
        .expect("run fangd --version");

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("fangd {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!state.exists(), "version command created {}", state.display());
    assert!(!socket.exists(), "version command created {}", socket.display());
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p fangd --test integration version_flag_exits_without_creating_runtime_state -- --exact
```

Expected: FAIL because `fangd` reports `unknown argument: --version` with exit status 2.

- [ ] **Step 3: Implement the minimal version flag**

Add this arm before the help arm in `parse_args`:

```rust
"--version" | "-V" => {
    println!("fangd {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
}
```

Change the help usage line to:

```rust
USAGE: fangd [--version] [--mock] [--tcp ADDR] [--socket PATH] [--state PATH]\n\n\
```

Add this help line before `--mock`:

```rust
--version       print the daemon version and exit\n\
```

- [ ] **Step 4: Run focused and full daemon tests**

Run:

```bash
cargo test -p fangd --test integration version_flag_exits_without_creating_runtime_state -- --exact
cargo test -p fangd
```

Expected: the focused test passes; the full `fangd` test suite reports zero failures.

- [ ] **Step 5: Commit**

```bash
git add crates/fangd/src/main.rs crates/fangd/tests/integration.rs
git commit -m "feat(fangd): add version command"
```

---

### Task 2: Add both custom RPM manifests and desktop assets

**Files:**
- Create: `packaging/rpm/metadata.test.mjs`
- Create: `packaging/rpm/fangd.spec`
- Create: `packaging/rpm/fang.spec`
- Create: `packaging/rpm/fang.sysusers`
- Create: `packaging/rpm/fang.desktop`

**Interfaces:**
- Consumes: staged files named `fangd`, `fang`, `fangd.service`, `fang.sysusers`, `fang.desktop`, `LICENSE`, and `fang-{32,128,256,512}.png`.
- Produces: RPM specs whose installed package names are exactly `fangd` and `fang`.

- [ ] **Step 1: Write the failing static metadata test**

Create `packaging/rpm/metadata.test.mjs`:

```javascript
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const read = (name) => fs.readFileSync(path.join(root, name), 'utf8');
const version = JSON.parse(read('app/package.json')).version;
const [major, minor] = version.split('.').map(Number);
const upper = `${major}.${minor + 1}.0`;

test('daemon spec uses native sysusers and a real license payload', () => {
  const spec = read('packaging/rpm/fangd.spec');
  assert.match(spec, new RegExp(`^Version:\\s*${version.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, /^Release:\s*1\s*$/m);
  assert.match(spec, /^License:\s*GPL-2\.0-only\s*$/m);
  assert.match(spec, /%\{_sysusersdir\}\/fang\.conf/);
  assert.match(spec, /%license %\{_licensedir\}\/%\{name\}\/LICENSE/);
  assert.match(spec, /%systemd_post fangd\.service/);
  assert.doesNotMatch(spec, /%sysusers_create_compat|groupadd|^%pre\s*$/m);
});

test('desktop spec owns strict daemon bounds and the tray runtime', () => {
  const spec = read('packaging/rpm/fang.spec');
  assert.match(spec, new RegExp(`^Version:\\s*${version.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, new RegExp(`^%global fangd_upper ${upper.replaceAll('.', '\\.')}\\s*$`, 'm'));
  assert.match(spec, /^Requires:\s*fangd >= %\{version\}\s*$/m);
  assert.match(spec, /^Requires:\s*fangd < %\{fangd_upper\}\s*$/m);
  assert.match(spec, /^Requires:\s*libayatana-appindicator-gtk3\s*$/m);
  assert.match(spec, /%license %\{_licensedir\}\/%\{name\}\/LICENSE/);
  assert.doesNotMatch(spec, /AutoReqProv:\s*no/);
});

test('sysusers and desktop files expose the required identities', () => {
  assert.equal(read('packaging/rpm/fang.sysusers'), 'g fang - -\n');
  const desktop = read('packaging/rpm/fang.desktop');
  assert.match(desktop, /^\[Desktop Entry\]$/m);
  assert.match(desktop, /^Exec=fang$/m);
  assert.match(desktop, /^Icon=fang$/m);
  assert.match(desktop, /^Terminal=false$/m);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
node --test packaging/rpm/metadata.test.mjs
```

Expected: FAIL with `ENOENT` for `packaging/rpm/fangd.spec`.

- [ ] **Step 3: Create the daemon spec and sysusers declaration**

Create `packaging/rpm/fang.sysusers`:

```text
g fang - -
```

Create `packaging/rpm/fangd.spec`:

```spec
%global debug_package %{nil}

Name: fangd
Version: 0.9.2
Release: 1
Summary: Privileged hardware-control daemon for Fang
License: GPL-2.0-only
URL: https://github.com/bladeandsoulx/fang-razer-linux
Source0: fangd
Source1: fangd.service
Source2: fang.sysusers
Source3: LICENSE
BuildRequires: systemd-rpm-macros
Requires: systemd

%description
Privileged daemon exposing performance, fan, lighting, power, and telemetry
controls for supported Razer Blade laptops over a local Unix socket.

%prep

%build

%install
install -Dpm0755 %{SOURCE0} %{buildroot}%{_bindir}/fangd
install -Dpm0644 %{SOURCE1} %{buildroot}%{_unitdir}/fangd.service
install -Dpm0644 %{SOURCE2} %{buildroot}%{_sysusersdir}/fang.conf
install -Dpm0644 %{SOURCE3} %{buildroot}%{_licensedir}/%{name}/LICENSE

%post
%systemd_post fangd.service

%preun
%systemd_preun fangd.service

%postun
%systemd_postun_with_restart fangd.service

%files
%license %{_licensedir}/%{name}/LICENSE
%{_bindir}/fangd
%{_unitdir}/fangd.service
%{_sysusersdir}/fang.conf
```

- [ ] **Step 4: Create the desktop entry and desktop spec**

Create `packaging/rpm/fang.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Fang
GenericName=Razer Blade Control Center
Comment=Control performance, fans, lighting, power, and displays
Exec=fang
Icon=fang
Terminal=false
Categories=System;Utility;
StartupNotify=true
```

Create `packaging/rpm/fang.spec`:

```spec
%global debug_package %{nil}
%global fangd_upper 0.10.0

Name: fang
Version: 0.9.2
Release: 1
Summary: Razer Blade control center for Linux
License: GPL-2.0-only
URL: https://github.com/bladeandsoulx/fang-razer-linux
Source0: fang
Source1: fang.desktop
Source2: LICENSE
Source3: fang-32.png
Source4: fang-128.png
Source5: fang-256.png
Source6: fang-512.png
BuildRequires: desktop-file-utils
Requires: fangd >= %{version}
Requires: fangd < %{fangd_upper}
Requires: libayatana-appindicator-gtk3

%description
Native desktop control center for performance modes, fan curves, lighting,
power, displays, and live telemetry on supported Razer Blade laptops.

%prep

%build

%install
install -Dpm0755 %{SOURCE0} %{buildroot}%{_bindir}/fang
install -Dpm0644 %{SOURCE1} %{buildroot}%{_datadir}/applications/fang.desktop
install -Dpm0644 %{SOURCE2} %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm0644 %{SOURCE3} %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/fang.png
install -Dpm0644 %{SOURCE4} %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/fang.png
install -Dpm0644 %{SOURCE5} %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/fang.png
install -Dpm0644 %{SOURCE6} %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/fang.png

%check
desktop-file-validate %{SOURCE1}

%files
%license %{_licensedir}/%{name}/LICENSE
%{_bindir}/fang
%{_datadir}/applications/fang.desktop
%{_datadir}/icons/hicolor/32x32/apps/fang.png
%{_datadir}/icons/hicolor/128x128/apps/fang.png
%{_datadir}/icons/hicolor/256x256/apps/fang.png
%{_datadir}/icons/hicolor/512x512/apps/fang.png
```

- [ ] **Step 5: Run the metadata test**

Run:

```bash
node --test packaging/rpm/metadata.test.mjs
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add packaging/rpm/fang.spec packaging/rpm/fangd.spec packaging/rpm/fang.desktop packaging/rpm/fang.sysusers packaging/rpm/metadata.test.mjs
git commit -m "feat(packaging): add Fedora RPM manifests"
```

---

### Task 3: Synchronize RPM versions with every release manifest

**Files:**
- Create: `app/scripts/version.test.mjs`
- Modify: `app/scripts/version.mjs`

**Interfaces:**
- Consumes: `Version:` in both specs and `%global fangd_upper` in `fang.spec`.
- Produces: `node app/scripts/version.mjs check` mismatch failures and `set VERSION` updates for both RPM specs.

- [ ] **Step 1: Write isolated failing CLI tests**

Create `app/scripts/version.test.mjs`:

```javascript
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const files = [
  'Cargo.toml',
  'Cargo.lock',
  'CHANGELOG.md',
  'app/package.json',
  'app/package-lock.json',
  'app/scripts/version.mjs',
  'app/src-tauri/Cargo.toml',
  'app/src-tauri/Cargo.lock',
  'app/src-tauri/tauri.conf.json',
  'packaging/rpm/fang.spec',
  'packaging/rpm/fangd.spec'
];

function fixture() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'fang-version-'));
  for (const name of files) {
    const destination = path.join(dir, name);
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.copyFileSync(path.join(root, name), destination);
  }
  return dir;
}

function run(dir, ...args) {
  return spawnSync(process.execPath, ['app/scripts/version.mjs', ...args], {
    cwd: dir,
    encoding: 'utf8'
  });
}

test('check rejects an incorrect RPM upper bound', () => {
  const dir = fixture();
  const spec = path.join(dir, 'packaging/rpm/fang.spec');
  fs.writeFileSync(spec, fs.readFileSync(spec, 'utf8').replace('fangd_upper 0.10.0', 'fangd_upper 0.11.0'));
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /RPM.*release line|fangd_upper/i);
  fs.rmSync(dir, { recursive: true });
});

test('set updates both RPM versions and the next-minor upper bound', () => {
  const dir = fixture();
  const result = run(dir, 'set', '0.9.3');
  assert.equal(result.status, 0, result.stdout + result.stderr);
  for (const name of ['packaging/rpm/fang.spec', 'packaging/rpm/fangd.spec']) {
    assert.match(fs.readFileSync(path.join(dir, name), 'utf8'), /^Version:\s*0\.9\.3$/m);
  }
  assert.match(
    fs.readFileSync(path.join(dir, 'packaging/rpm/fang.spec'), 'utf8'),
    /^%global fangd_upper 0\.10\.0$/m
  );
  fs.rmSync(dir, { recursive: true });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
node --test app/scripts/version.test.mjs
```

Expected: both tests fail because the current script neither checks nor updates RPM specs.

- [ ] **Step 3: Add RPM field helpers and checks**

Add to `app/scripts/version.mjs` after `replaceCargoPackageVersion`:

```javascript
function rpmField(text, field) {
  return capture('RPM ' + field, text, new RegExp('^' + field + ':\\s*(\\S+)\\s*$', 'm'));
}

function rpmMacro(text, name) {
  return capture('RPM macro ' + name, text, new RegExp('^%global\\s+' + name + '\\s+(\\S+)\\s*$', 'm'));
}

function replaceRpmField(text, field, value) {
  return replaceRequired(
    'RPM ' + field,
    text,
    new RegExp('^(' + field + ':\\s*)\\S+(\\s*)$', 'm'),
    '$1' + value + '$2'
  );
}

function replaceRpmMacro(text, name, value) {
  return replaceRequired(
    'RPM macro ' + name,
    text,
    new RegExp('^(%global\\s+' + name + '\\s+)\\S+(\\s*)$', 'm'),
    '$1' + value + '$2'
  );
}
```

In `currentVersions`, read both specs and append:

```javascript
const fangRpm = read('packaging/rpm/fang.spec');
const fangdRpm = read('packaging/rpm/fangd.spec');
```

```javascript
['fang RPM spec', rpmField(fangRpm, 'Version')],
['fangd RPM spec', rpmField(fangdRpm, 'Version')],
```

In `check`, after calculating `upper`, add:

```javascript
const fangRpm = read('packaging/rpm/fang.spec');
if (
  !/^Requires:\s*fangd >= %\{version\}\s*$/m.test(fangRpm) ||
  !/^Requires:\s*fangd < %\{fangd_upper\}\s*$/m.test(fangRpm) ||
  rpmMacro(fangRpm, 'fangd_upper') !== upper
) {
  throw new Error('Fang RPM must depend on the matching fangd release line');
}
```

- [ ] **Step 4: Update RPM fields in `setVersion`**

After writing `tauri.conf.json`, add:

```javascript
for (const name of ['packaging/rpm/fang.spec', 'packaging/rpm/fangd.spec']) {
  text = replaceRpmField(read(name), 'Version', version);
  if (name.endsWith('/fang.spec')) {
    text = replaceRpmMacro(text, 'fangd_upper', major + '.' + (minor + 1) + '.0');
  }
  write(name, text);
}
```

- [ ] **Step 5: Run version and packaging tests**

Run:

```bash
node --test app/scripts/version.test.mjs packaging/rpm/metadata.test.mjs
node app/scripts/version.mjs check
```

Expected: 5 tests pass and the check prints `Fang version sync OK: 0.9.2`.

- [ ] **Step 6: Commit**

```bash
git add app/scripts/version.mjs app/scripts/version.test.mjs
git commit -m "test(release): synchronize RPM versions"
```

---

### Task 4: Add the reproducible Fedora RPM build entry point

**Files:**
- Create: `packaging/rpm/build-script.test.mjs`
- Create: `packaging/rpm/build.sh`

**Interfaces:**
- Consumes: repository source plus Fedora build tools.
- Produces: exactly two binary RPMs in the directory passed as argument, defaulting to `target/rpm-dist`.

- [ ] **Step 1: Write the failing build-script contract test**

Create `packaging/rpm/build-script.test.mjs`:

```javascript
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

test('RPM build script uses custom specs and disables Tauri bundling', () => {
  const script = fs.readFileSync(path.join(root, 'packaging/rpm/build.sh'), 'utf8');
  assert.match(script, /node app\/scripts\/version\.mjs check/);
  assert.match(script, /cargo build --release -p fangd/);
  assert.match(script, /npm run tauri build -- --no-bundle/);
  assert.match(script, /rpmbuild .*fangd\.spec/s);
  assert.match(script, /rpmbuild .*fang\.spec/s);
  assert.doesNotMatch(script, /--bundles rpm/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
node --test packaging/rpm/build-script.test.mjs
```

Expected: FAIL with `ENOENT` for `packaging/rpm/build.sh`.

- [ ] **Step 3: Create the build script**

Create executable `packaging/rpm/build.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT="${1:-$ROOT/target/rpm-dist}"
TOPDIR="$(mktemp -d)"
trap 'rm -rf "$TOPDIR"' EXIT

for command in cargo node npm rpmbuild rpm; do
  command -v "$command" >/dev/null || {
    echo "missing build command: $command" >&2
    exit 1
  }
done

cd "$ROOT"
node app/scripts/version.mjs check
cargo build --release -p fangd
(
  cd app
  npm ci
  npm run tauri build -- --no-bundle
)

mkdir -p "$TOPDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
install -pm0755 target/release/fangd "$TOPDIR/SOURCES/fangd"
install -pm0755 app/src-tauri/target/release/fang "$TOPDIR/SOURCES/fang"
install -pm0644 packaging/fangd.service "$TOPDIR/SOURCES/fangd.service"
install -pm0644 packaging/rpm/fang.sysusers "$TOPDIR/SOURCES/fang.sysusers"
install -pm0644 packaging/rpm/fang.desktop "$TOPDIR/SOURCES/fang.desktop"
install -pm0644 LICENSE "$TOPDIR/SOURCES/LICENSE"
install -pm0644 app/src-tauri/icons/32x32.png "$TOPDIR/SOURCES/fang-32.png"
install -pm0644 app/src-tauri/icons/128x128.png "$TOPDIR/SOURCES/fang-128.png"
install -pm0644 app/src-tauri/icons/128x128@2x.png "$TOPDIR/SOURCES/fang-256.png"
install -pm0644 app/src-tauri/icons/icon.png "$TOPDIR/SOURCES/fang-512.png"

rpmbuild --define "_topdir $TOPDIR" -bb packaging/rpm/fangd.spec
rpmbuild --define "_topdir $TOPDIR" -bb packaging/rpm/fang.spec

mkdir -p "$OUTPUT"
find "$OUTPUT" -maxdepth 1 -type f -name '*.rpm' -delete
mapfile -t built < <(find "$TOPDIR/RPMS" -type f -name '*.rpm' -print | sort)
[[ "${#built[@]}" -eq 2 ]] || {
  printf 'expected two RPMs, found %s\n' "${#built[@]}" >&2
  exit 1
}

declare -A seen=()
for package in "${built[@]}"; do
  name="$(rpm -qp --queryformat '%{NAME}' "$package")"
  [[ "$name" == "fang" || "$name" == "fangd" ]] || {
    echo "unexpected RPM package: $name" >&2
    exit 1
  }
  [[ -z "${seen[$name]:-}" ]] || {
    echo "duplicate RPM package: $name" >&2
    exit 1
  }
  seen[$name]=1
  install -pm0644 "$package" "$OUTPUT/"
done

[[ -n "${seen[fang]:-}" && -n "${seen[fangd]:-}" ]]
printf 'RPM artifacts:\n'
find "$OUTPUT" -maxdepth 1 -type f -name '*.rpm' -printf '%f\n' | sort
```

Run:

```bash
chmod +x packaging/rpm/build.sh
```

- [ ] **Step 4: Run host-runnable script checks**

Run:

```bash
bash -n packaging/rpm/build.sh
node --test packaging/rpm/build-script.test.mjs packaging/rpm/metadata.test.mjs
```

Expected: Bash syntax succeeds and 4 Node tests pass. The actual RPM build is exercised in the Fedora 43 jobs in Tasks 6 and 7.

- [ ] **Step 5: Commit**

```bash
git add packaging/rpm/build.sh packaging/rpm/build-script.test.mjs
git commit -m "build(rpm): add Fedora package builder"
```

---

### Task 5: Add installed-package verification

**Files:**
- Create: `packaging/rpm/mock_smoke.py`
- Create: `packaging/rpm/mock_smoke_test.py`
- Create: `packaging/rpm/verify.sh`

**Interfaces:**
- Consumes: a directory containing exactly one `fang` RPM and one `fangd` RPM.
- Produces: exit zero only after metadata, incompatible-version, DNF install, sysusers, systemd unit, daemon mock, desktop launch, and removal checks pass.

- [ ] **Step 1: Write the failing response-parser test**

Create `packaging/rpm/mock_smoke_test.py`:

```python
import importlib.util
import io
import pathlib
import unittest

MODULE_PATH = pathlib.Path(__file__).with_name("mock_smoke.py")
SPEC = importlib.util.spec_from_file_location("mock_smoke", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ResponseTest(unittest.TestCase):
    def test_skips_events_and_returns_matching_response(self):
        stream = io.BytesIO(
            b'{"event":"telemetry","data":{}}\n'
            b'{"id":1,"ok":true,"data":{"mock":true}}\n'
        )
        response = MODULE.read_response(stream, 1)
        self.assertTrue(response["ok"])
        self.assertTrue(response["data"]["mock"])


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
python3 -m unittest packaging/rpm/mock_smoke_test.py
```

Expected: FAIL because `mock_smoke.py` does not exist.

- [ ] **Step 3: Implement the installed-daemon smoke client**

Create `packaging/rpm/mock_smoke.py`:

```python
#!/usr/bin/env python3
import json
import pathlib
import socket
import subprocess
import sys
import tempfile
import time


def read_response(stream, request_id):
    while True:
        line = stream.readline()
        if not line:
            raise RuntimeError("fangd closed the connection before responding")
        message = json.loads(line)
        if message.get("id") == request_id:
            return message


def main():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    with tempfile.TemporaryDirectory(prefix="fangd-rpm-smoke-") as directory:
        state = pathlib.Path(directory) / "state.json"
        process = subprocess.Popen(
            [
                "/usr/bin/fangd",
                "--mock",
                "--tcp",
                f"127.0.0.1:{port}",
                "--state",
                str(state),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        try:
            connection = None
            for _ in range(50):
                try:
                    connection = socket.create_connection(("127.0.0.1", port), timeout=1)
                    break
                except OSError:
                    if process.poll() is not None:
                        raise RuntimeError(process.stderr.read().decode())
                    time.sleep(0.1)
            if connection is None:
                raise RuntimeError("installed fangd did not listen within five seconds")

            with connection:
                stream = connection.makefile("rwb")
                stream.write(b'{"id":1,"cmd":"get_status"}\n')
                stream.flush()
                response = read_response(stream, 1)
                if response.get("ok") is not True or response.get("data", {}).get("mock") is not True:
                    raise RuntimeError(f"unexpected response: {response}")
        finally:
            if process.poll() is None:
                process.terminate()
            try:
                status = process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise RuntimeError("installed fangd ignored SIGTERM")
            if status != 0 and sys.exc_info()[0] is None:
                raise RuntimeError(f"installed fangd exited with {status}: {process.stderr.read().decode()}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run the parser test**

Run:

```bash
python3 -m unittest packaging/rpm/mock_smoke_test.py
```

Expected: 1 test passes.

- [ ] **Step 5: Create the package verifier**

Create executable `packaging/rpm/verify.sh` with these required phases:

```bash
#!/usr/bin/env bash
set -euo pipefail

RPM_DIR="${1:?usage: verify.sh RPM_DIRECTORY}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mapfile -t packages < <(find "$RPM_DIR" -maxdepth 1 -type f -name '*.rpm' -print | sort)
[[ "${#packages[@]}" -eq 2 ]] || {
  printf 'expected two RPMs, found %s\n' "${#packages[@]}" >&2
  exit 1
}

fang=
fangd=
for package in "${packages[@]}"; do
  name="$(rpm -qp --queryformat '%{NAME}' "$package")"
  case "$name" in
    fang) fang="$package" ;;
    fangd) fangd="$package" ;;
    *) echo "unexpected package name: $name" >&2; exit 1 ;;
  esac
done
[[ -n "$fang" && -n "$fangd" ]]

version="$(rpm -qp --queryformat '%{VERSION}' "$fang")"
upper="$(sed -n 's/^%global fangd_upper //p' "$ROOT/packaging/rpm/fang.spec")"
for package in "$fang" "$fangd"; do
  [[ "$(rpm -qp --queryformat '%{VERSION}' "$package")" == "$version" ]]
  [[ "$(rpm -qp --queryformat '%{RELEASE}' "$package")" == "1" ]]
  [[ "$(rpm -qp --queryformat '%{ARCH}' "$package")" == "x86_64" ]]
  [[ "$(rpm -qp --queryformat '%{LICENSE}' "$package")" == "GPL-2.0-only" ]]
done

rpm -qlp "$fangd" | grep -Fx /usr/bin/fangd
rpm -qlp "$fangd" | grep -Fx /usr/lib/systemd/system/fangd.service
rpm -qlp "$fangd" | grep -Fx /usr/lib/sysusers.d/fang.conf
rpm -qlp "$fangd" | grep -Fx /usr/share/licenses/fangd/LICENSE
rpm -qp --queryformat '[%{SYSUSERS}\n]' "$fangd" | grep -F 'g fang - -'
if rpm -qp --scripts "$fangd" | grep -E 'sysusers_create_compat|groupadd'; then
  echo "daemon RPM contains legacy group creation" >&2
  exit 1
fi

rpm -qlp "$fang" | grep -Fx /usr/bin/fang
rpm -qlp "$fang" | grep -Fx /usr/share/applications/fang.desktop
rpm -qlp "$fang" | grep -Fx /usr/share/licenses/fang/LICENSE
for size in 32 128 256 512; do
  rpm -qlp "$fang" | grep -Fx "/usr/share/icons/hicolor/${size}x${size}/apps/fang.png"
done
rpm -qp --requires "$fang" | grep -Fx "fangd >= $version"
rpm -qp --requires "$fang" | grep -Fx "fangd < $upper"
rpm -qp --requires "$fang" | grep -Fx libayatana-appindicator-gtk3

make_dummy_fangd() {
  local dummy_version="$1"
  local top="$TMP/dummy-$dummy_version"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  {
    printf 'Name: fangd\nVersion: %s\nRelease: 1\n' "$dummy_version"
    printf 'Summary: dependency-bound test double\nLicense: MIT\nBuildArch: noarch\n'
    printf '%%description\nDependency-bound test double.\n'
    printf '%%install\nmkdir -p %%{buildroot}/usr/share/fang-rpm-test\n'
    printf 'echo %s > %%{buildroot}/usr/share/fang-rpm-test/%s\n' "$dummy_version" "$dummy_version"
    printf '%%files\n/usr/share/fang-rpm-test/%s\n' "$dummy_version"
  } > "$top/SPECS/fangd.spec"
  rpmbuild --define "_topdir $top" -bb "$top/SPECS/fangd.spec" >/dev/null
  find "$top/RPMS" -type f -name '*.rpm' -print -quit
}

for incompatible in 0.0.1 "$upper"; do
  dummy="$(make_dummy_fangd "$incompatible")"
  if dnf install -y --setopt=tsflags=test "$fang" "$dummy" >"$TMP/dnf-$incompatible.log" 2>&1; then
    echo "fang accepted incompatible fangd $incompatible" >&2
    cat "$TMP/dnf-$incompatible.log" >&2
    exit 1
  fi
done

dnf install -y "$fangd" "$fang"
[[ "$(rpm -q --queryformat '%{VERSION}' fang)" == "$version" ]]
[[ "$(rpm -q --queryformat '%{VERSION}' fangd)" == "$version" ]]
getent group fang
rpm -V fangd fang
/usr/bin/fangd --version | grep -Fx "fangd $version"
systemd-analyze verify /usr/lib/systemd/system/fangd.service
python3 "$ROOT/packaging/rpm/mock_smoke.py"
desktop-file-validate /usr/share/applications/fang.desktop
if ldd /usr/bin/fang | grep -F 'not found'; then
  echo "desktop binary has unresolved libraries" >&2
  exit 1
fi

set +e
dbus-run-session -- timeout 8s xvfb-run -a /usr/bin/fang >"$TMP/fang.out" 2>"$TMP/fang.err"
desktop_status=$?
set -e
if [[ "$desktop_status" -ne 124 ]]; then
  cat "$TMP/fang.out" "$TMP/fang.err" >&2
  echo "desktop exited before smoke timeout: $desktop_status" >&2
  exit 1
fi

while IFS= read -r path; do
  if [[ -f "$path" || -L "$path" ]]; then
    printf '%s\n' "$path"
  fi
done < <(rpm -ql fang fangd) > "$TMP/owned-files"

dnf remove -y fang fangd
while IFS= read -r path; do
  [[ ! -e "$path" && ! -L "$path" ]] || {
    echo "packaged file remains after removal: $path" >&2
    exit 1
  }
done < "$TMP/owned-files"

printf 'Fedora RPM verification passed on %s\n' "$(rpm -E '%{fedora}')"
```

Run:

```bash
chmod +x packaging/rpm/mock_smoke.py packaging/rpm/verify.sh
```

- [ ] **Step 6: Run host-runnable verifier checks**

Run:

```bash
bash -n packaging/rpm/verify.sh
python3 -m unittest packaging/rpm/mock_smoke_test.py
node --test packaging/rpm/metadata.test.mjs
```

Expected: Bash syntax succeeds, the Python test passes, and 3 metadata tests pass.

- [ ] **Step 7: Commit**

```bash
git add packaging/rpm/mock_smoke.py packaging/rpm/mock_smoke_test.py packaging/rpm/verify.sh
git commit -m "test(rpm): verify Fedora package lifecycle"
```

---

### Task 6: Build and test RPMs on every pull request

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `packaging/rpm/build.sh` and `packaging/rpm/verify.sh`.
- Produces: a `fang-rpms` workflow artifact and Fedora 43/44 required checks.

- [ ] **Step 1: Run a workflow syntax baseline**

Run:

```bash
python3 - <<'PY'
import yaml
yaml.safe_load(open(".github/workflows/ci.yml", encoding="utf-8"))
print("CI YAML OK")
PY
```

Expected: `CI YAML OK`.

- [ ] **Step 2: Add the Fedora 43 build job**

Append this job under `jobs` in `.github/workflows/ci.yml`:

```yaml
  rpm-build:
    name: build RPMs (Fedora 43)
    runs-on: ubuntu-latest
    container: fedora:43
    steps:
      - run: >
          dnf install -y
          git curl tar gzip gcc gcc-c++ make pkgconf-pkg-config systemd-devel
          webkit2gtk4.1-devel gtk3-devel librsvg2-devel
          libayatana-appindicator-gtk3-devel openssl-devel
          rpm-build desktop-file-utils findutils
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: app/package-lock.json
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: |
            .
            app/src-tauri
      - run: packaging/rpm/build.sh target/rpm-dist
      - uses: actions/upload-artifact@v4
        with:
          name: fang-rpms
          path: target/rpm-dist/*.rpm
          if-no-files-found: error
```

- [ ] **Step 3: Add the Fedora 43/44 install matrix**

Append:

```yaml
  rpm-test:
    name: test RPMs (${{ matrix.fedora }})
    needs: rpm-build
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        fedora: [43, 44]
    container:
      image: fedora:${{ matrix.fedora }}
    steps:
      - run: >
          dnf install -y
          git tar gzip rpm-build systemd systemd-rpm-macros desktop-file-utils
          python3 dbus-daemon xorg-x11-server-Xvfb xorg-x11-xauth
          libayatana-appindicator-gtk3 findutils procps-ng
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: fang-rpms
          path: target/rpm-dist
      - run: packaging/rpm/verify.sh target/rpm-dist
```

- [ ] **Step 4: Parse and inspect the workflow**

Run:

```bash
python3 - <<'PY'
import yaml
data = yaml.safe_load(open(".github/workflows/ci.yml", encoding="utf-8"))
assert sorted(data["jobs"]) == ["app", "daemon", "rpm-build", "rpm-test"]
print(*sorted(data["jobs"]))
PY
```

Expected output contains `app`, `daemon`, `rpm-build`, and `rpm-test`.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: build and test Fedora RPMs"
```

---

### Task 7: Gate GitHub releases on both package families

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: successful DEB build, successful RPM build, and both RPM matrix checks.
- Produces: one draft release containing exactly two DEBs and two RPMs.

- [ ] **Step 1: Write the intended job graph assertion**

Run this before editing:

```bash
python3 - <<'PY'
import yaml
data = yaml.safe_load(open(".github/workflows/release.yml", encoding="utf-8"))
assert sorted(data["jobs"]) == ["debs", "draft-release", "rpm-test", "rpms"], "missing gated release graph"
PY
```

Expected: FAIL with `missing gated release graph`.

- [ ] **Step 2: Replace the release workflow with the gated graph**

Replace `.github/workflows/release.yml` with:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  debs:
    name: build .deb packages
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - uses: dtolnay/rust-toolchain@stable
      - run: >
          sudo apt-get update && sudo apt-get install -y --no-install-recommends
          libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev
          libayatana-appindicator3-dev libudev-dev
      - uses: Swatinem/rust-cache@v2
      - name: Build fangd .deb
        run: |
          cargo install cargo-deb --locked
          cargo deb -p fangd
      - name: Build app .deb
        working-directory: app
        run: |
          npm ci
          npm run tauri build -- --bundles deb
      - uses: actions/upload-artifact@v4
        with:
          name: fang-debs
          path: |
            target/debian/*.deb
            app/src-tauri/target/release/bundle/deb/*.deb
          if-no-files-found: error

  rpms:
    name: build RPMs (Fedora 43)
    runs-on: ubuntu-latest
    container: fedora:43
    steps:
      - run: >
          dnf install -y
          git curl tar gzip gcc gcc-c++ make pkgconf-pkg-config systemd-devel
          webkit2gtk4.1-devel gtk3-devel librsvg2-devel
          libayatana-appindicator-gtk3-devel openssl-devel
          rpm-build desktop-file-utils findutils
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: app/package-lock.json
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: |
            .
            app/src-tauri
      - run: packaging/rpm/build.sh target/rpm-dist
      - uses: actions/upload-artifact@v4
        with:
          name: fang-rpms
          path: target/rpm-dist/*.rpm
          if-no-files-found: error

  rpm-test:
    name: test RPMs (${{ matrix.fedora }})
    needs: rpms
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        fedora: [43, 44]
    container:
      image: fedora:${{ matrix.fedora }}
    steps:
      - run: >
          dnf install -y
          git tar gzip rpm-build systemd systemd-rpm-macros desktop-file-utils
          python3 dbus-daemon xorg-x11-server-Xvfb xorg-x11-xauth
          libayatana-appindicator-gtk3 findutils procps-ng
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: fang-rpms
          path: target/rpm-dist
      - run: packaging/rpm/verify.sh target/rpm-dist

  draft-release:
    name: create four-package draft release
    needs: [debs, rpm-test]
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: fang-debs
          path: dist/deb
      - uses: actions/download-artifact@v4
        with:
          name: fang-rpms
          path: dist/rpm
      - name: Verify release inventory
        run: |
          sudo apt-get update
          sudo apt-get install -y rpm
          mapfile -t debs < <(find dist/deb -type f -name '*.deb' -print | sort)
          mapfile -t rpms < <(find dist/rpm -type f -name '*.rpm' -print | sort)
          test "${#debs[@]}" -eq 2
          test "${#rpms[@]}" -eq 2
          test "$(for package in "${debs[@]}"; do dpkg-deb -f "$package" Package; done | sort)" = $'fang\nfangd'
          test "$(for package in "${rpms[@]}"; do rpm -qp --queryformat '%{NAME}\n' "$package"; done | sort)" = $'fang\nfangd'
      - uses: softprops/action-gh-release@v2
        with:
          draft: true
          generate_release_notes: true
          fail_on_unmatched_files: true
          files: |
            dist/deb/*.deb
            dist/rpm/*.rpm
```

- [ ] **Step 3: Validate the release graph and release ordering**

Run:

```bash
python3 - <<'PY'
import yaml
data = yaml.safe_load(open(".github/workflows/release.yml", encoding="utf-8"))
assert sorted(data["jobs"]) == ["debs", "draft-release", "rpm-test", "rpms"]
assert data["jobs"]["draft-release"]["needs"] == ["debs", "rpm-test"]
assert data["jobs"]["rpm-test"]["needs"] == "rpms"
print("release graph OK")
PY
```

Expected: `release graph OK`.

Run:

```bash
rg -n 'softprops/action-gh-release' .github/workflows/release.yml
```

Expected: exactly one match, inside `draft-release`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): gate four-package releases"
```

---

### Task 8: Document Fedora installation and real-hardware limits

**Files:**
- Modify: `README.md`
- Modify: `HARDWARE_TESTING.md`
- Modify: `.github/ISSUE_TEMPLATE/bug_report.yml`
- Modify: `.github/ISSUE_TEMPLATE/model-support.yml`

**Interfaces:**
- Consumes: release filenames and systemd/sysusers behavior from Tasks 2 and 7.
- Produces: copy-paste Fedora setup, uninstall, diagnostics, and community validation instructions.

- [ ] **Step 1: Write documentation presence checks**

Run before editing:

```bash
rg -n -F '## Install (Fedora 43 / 44)' README.md
rg -n -F 'sudo dnf install ./fangd-*.rpm ./fang-*.rpm' README.md HARDWARE_TESTING.md
rg -n -F 'SELinux status' README.md HARDWARE_TESTING.md
```

Expected: no matches and nonzero exit status.

- [ ] **Step 2: Add the Fedora section to `README.md`**

Immediately after the Ubuntu/Debian prebuilt instructions, add:

````markdown
## Install (Fedora 43 / 44)

GitHub releases include x86_64 RPMs for the daemon and desktop app. Download
both files from the same release, then install and activate them:

```sh
sudo dnf install ./fangd-*.rpm ./fang-*.rpm
sudo systemctl enable --now fangd
sudo usermod -aG fang "$USER"
```

Log out and back in once after `usermod`; the desktop app needs the new group
membership to access `/run/fangd.sock`. Diagnose the daemon with
`systemctl status fangd` and `journalctl -u fangd`. Remove both packages with:

```sh
sudo dnf remove fang fangd
```

These first RPMs are unsigned direct downloads rather than a configured DNF
repository. Fedora package installation is tested in Fedora 43/44 containers;
physical Razer hardware and SELinux-enforcing behavior depend on community
validation through `HARDWARE_TESTING.md`.
````

- [ ] **Step 3: Make the hardware checklist distribution-aware**

Change the title to:

```markdown
# First run on real hardware (Razer Blade, Ubuntu or Fedora)
```

In baseline, add:

```sh
cat /etc/os-release             # record distribution and version
echo "${XDG_SESSION_TYPE:-?}"   # record Wayland or X11
getenforce 2>/dev/null || true  # Fedora: record enforcing/permissive/disabled
```

Replace the command block in hardware-checklist step 1 with:

````markdown
Use the package path for the distribution:

```sh
# Ubuntu/Debian source build:
sudo ./packaging/install.sh

# Fedora 43/44 prebuilt release packages:
sudo dnf install ./fangd-*.rpm ./fang-*.rpm
sudo systemctl enable --now fangd
sudo usermod -aG fang "$USER"  # log out and back in once
```

Then inspect the daemon:

```sh
journalctl -u fangd -b --no-pager | tail -20
```
````

In rollback, include:

```sh
sudo dnf remove fang fangd       # Fedora RPM installs
```

Replace the reporting paragraph with:

```markdown
Open an issue with: model + year, distribution + version, desktop + Wayland/X11
session, `lsusb -d 1532:` output, `journalctl -u fangd -b` snippet, and which
steps passed or failed. On Fedora, also include `getenforce` and any Fang-related
`ausearch -m AVC -ts recent` denials. This is enough to distinguish packaging,
SELinux, desktop-session, and hardware-profile failures.
```

- [ ] **Step 4: Update issue templates**

Change the bug-report system placeholder to:

```yaml
placeholder: "Blade 18 2023 · Fedora 44 or Ubuntu 24.04 · GNOME Wayland · SELinux enforcing"
```

Add these required inputs after `pid` in `model-support.yml`:

```yaml
  - type: input
    id: system
    attributes:
      label: Distribution + desktop session
      placeholder: "Fedora 44 · GNOME Wayland"
    validations:
      required: true
  - type: input
    id: selinux
    attributes:
      label: SELinux status and denials
      description: "Fedora: include `getenforce` and any relevant `ausearch -m AVC -ts recent` lines"
      placeholder: "Enforcing · no Fang-related AVC denials"
```

- [ ] **Step 5: Verify documentation and YAML**

Run:

```bash
rg -n -F '## Install (Fedora 43 / 44)' README.md
rg -n -F 'sudo dnf install ./fangd-*.rpm ./fang-*.rpm' README.md HARDWARE_TESTING.md
rg -n -F 'SELinux status' README.md HARDWARE_TESTING.md .github/ISSUE_TEMPLATE
python3 - <<'PY'
import glob
import yaml
for filename in glob.glob(".github/ISSUE_TEMPLATE/*.yml"):
    yaml.safe_load(open(filename, encoding="utf-8"))
print("issue YAML OK")
PY
```

Expected: all required phrases are found and output ends with `issue YAML OK`.

- [ ] **Step 6: Commit**

```bash
git add README.md HARDWARE_TESTING.md .github/ISSUE_TEMPLATE/bug_report.yml .github/ISSUE_TEMPLATE/model-support.yml
git commit -m "docs: add Fedora installation and testing"
```

---

### Task 9: Prepare, validate, push, and publish v0.9.3

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `app/package.json`
- Modify: `app/package-lock.json`
- Modify: `app/src-tauri/Cargo.toml`
- Modify: `app/src-tauri/Cargo.lock`
- Modify: `app/src-tauri/tauri.conf.json`
- Modify: `packaging/rpm/fang.spec`
- Modify: `packaging/rpm/fangd.spec`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: all implementation tasks and a clean GitHub-authenticated repository.
- Produces: published GitHub release `v0.9.3` with four exact package assets.

- [ ] **Step 1: Bump every machine-managed version**

Run:

```bash
node app/scripts/version.mjs set 0.9.3
```

Expected: manifests, lockfiles, DEB bounds, RPM `Version` fields, and
`fangd_upper` update; the upper bound remains `0.10.0`.

- [ ] **Step 2: Add the changelog release**

Insert above the current first release heading in `CHANGELOG.md`:

```markdown
## [0.9.3] — 2026-07-18 — Fedora RPM support

### Added

- Native x86_64 RPM packages for Fedora 43 and Fedora 44.
- Fedora 43/44 package build, install, launch, dependency, and removal checks.
- `fangd --version` for package and diagnostic verification.

### Changed

- GitHub releases are created only after both DEBs and both RPMs pass their
  release gates.
```

Add this reference before the existing `[0.9.2]` link at the bottom:

```markdown
[0.9.3]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.3
```

- [ ] **Step 3: Run the complete local verification suite**

Run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node --test app/scripts/version.test.mjs packaging/rpm/*.test.mjs
python3 -m unittest packaging/rpm/mock_smoke_test.py
bash -n packaging/rpm/build.sh packaging/rpm/verify.sh
node app/scripts/version.mjs check
npm test --prefix app
npm run build --prefix app
cargo fmt --check --manifest-path app/src-tauri/Cargo.toml
cargo clippy --manifest-path app/src-tauri/Cargo.toml --bin fang --all-targets -- -D warnings
cargo test --manifest-path app/src-tauri/Cargo.toml --bin fang
python3 - <<'PY'
import yaml
for filename in [".github/workflows/ci.yml", ".github/workflows/release.yml"]:
    yaml.safe_load(open(filename, encoding="utf-8"))
print("workflow YAML OK")
PY
git diff --check
```

Expected: every command exits zero, all tests report zero failures, and the
workflow parser prints `workflow YAML OK`.

- [ ] **Step 4: Commit the release bump**

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md app/package.json app/package-lock.json app/src-tauri/Cargo.toml app/src-tauri/Cargo.lock app/src-tauri/tauri.conf.json packaging/rpm/fang.spec packaging/rpm/fangd.spec
git commit -m "feat: release Fang 0.9.3 with Fedora RPMs"
```

- [ ] **Step 5: Push `main` and require green CI**

Run:

```bash
git push origin main
run_id="$(gh run list --workflow ci.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$run_id" --exit-status
```

Expected: the run exits zero with `daemon`, `app`, `build RPMs (Fedora 43)`,
`test RPMs (43)`, and `test RPMs (44)` successful.

- [ ] **Step 6: Tag only the CI-verified commit**

Run:

```bash
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
git tag -a v0.9.3 -m "Fang 0.9.3"
git push origin v0.9.3
release_run="$(gh run list --workflow release.yml --branch v0.9.3 --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$release_run" --exit-status
```

Expected: release workflow exits zero after DEBs, RPMs, Fedora 43/44 RPM tests,
and draft creation.

- [ ] **Step 7: Verify the draft inventory before publication**

Run:

```bash
gh release view v0.9.3 --json isDraft,assets --jq '{draft: .isDraft, assets: [.assets[].name] | sort}'
```

Expected:

```json
{
  "draft": true,
  "assets": [
    "Fang_0.9.3_amd64.deb",
    "fang-0.9.3-1.x86_64.rpm",
    "fangd-0.9.3-1.x86_64.rpm",
    "fangd_0.9.3-1_amd64.deb"
  ]
}
```

- [ ] **Step 8: Publish and verify the release**

Run:

```bash
gh release edit v0.9.3 --draft=false
gh release view v0.9.3 --json isDraft,isPrerelease,tagName,url,assets
```

Expected: `isDraft` and `isPrerelease` are false, `tagName` is `v0.9.3`, and
the same four package assets remain attached.
