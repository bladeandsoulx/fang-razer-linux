# Neon Fang Installer Banner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the interactive release installer's bordered heading with the approved static, horizontally composed `V FANG` neon terminal banner.

**Architecture:** `install.sh` remains the only runtime owner of the streamed banner and renders it from explicit mark, wordmark, HUD, and metadata color roles. `packaging/installer/banner.txt` remains the exact no-color snapshot, installer fixtures compare colored and uncolored output against it, and `app/scripts/version.mjs` keeps its embedded version synchronized during future release bumps.

**Tech Stack:** Bash, ANSI 4-bit SGR colors, Node.js built-in test runner, ShellCheck

## Global Constraints

- The published `v0.9.4` release is immutable and remains unchanged.
- This source change lands on `main` and first ships in the next release cut from it.
- The banner is horizontally arranged as `V FANG`; there is no outer box or separate emblem above the name.
- The mark is bold bright green, FANG is bold bright white, the HUD is bold bright cyan, and metadata is normal white.
- `NO_COLOR` must produce the exact checked-in monochrome snapshot with no escape sequences.
- Noninteractive output must contain neither the banner nor ANSI escape sequences.
- Every visible banner row must contain at most 72 Unicode code points.
- The banner is static: no animation, cursor movement, screen clearing, timing delays, or sound.
- Do not add FIGlet, `tput`, Python, Node, or any other runtime dependency to `install.sh`.
- The metadata version must come from the existing `VERSION`; `x86_64` remains literal because unsupported architectures are rejected.
- Preserve the existing green, cyan, yellow, and red progress roles and every installer safety, download, verification, elevation, package, service, and group-membership contract.
- Keep all executable statements inside functions except the final `main "$@"` call.
- Do not add this banner to the `0.9.4` changelog entries; it did not ship in that immutable release.
- Do not create, edit, publish, replace, or delete a GitHub release or tag in this implementation.
- Preserve the unrelated `docs/superpowers/plans/2026-07-23-beginner-friendly-readme.md` working-tree file and do not stage it.

## File Structure

- `packaging/installer/banner.txt` — exact current-version monochrome banner snapshot.
- `packaging/installer/installer.test.mjs` — terminal capability, palette, snapshot, version, width, and noninteractive behavior contracts.
- `install.sh` — runtime color-role configuration and streamed banner rendering.
- `app/scripts/version.mjs` — release-version reader and setter for the banner snapshot.
- `app/scripts/version.test.mjs` — isolated proof that version checks and bumps include the banner snapshot.

---

### Task 1: Render and verify the Neon Fang banner

**Files:**
- Modify: `packaging/installer/banner.txt`
- Modify: `packaging/installer/installer.test.mjs:657-689`
- Modify: `install.sh:24-60`

**Interfaces:**
- Consumes: `VERSION: string`, `OUTPUT_TTY: 0 | 1`, and the presence or absence of `NO_COLOR`.
- Produces: `COLOR_BANNER_MARK`, `COLOR_BANNER_WORDMARK`, `COLOR_BANNER_HUD`, and `COLOR_BANNER_METADATA` shell variables plus `print_banner(): shell status`.

- [ ] **Step 1: Replace the monochrome snapshot**

Replace `packaging/installer/banner.txt` byte-for-byte with:

```text
    ██╗   ██╗   ███████╗ █████╗ ███╗   ██╗ ██████╗
    ██║   ██║   ██╔════╝██╔══██╗████╗  ██║██╔════╝
    ╚██╗ ██╔╝   █████╗  ███████║██╔██╗ ██║██║  ███╗
     ╚████╔╝    ██╔══╝  ██╔══██║██║╚██╗██║██║   ██║
      ╚██╔╝     ██║     ██║  ██║██║ ╚████║╚██████╔╝
       ╚═╝      ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝

    ━━━ RAZER BLADE CONTROL // INSTALLER ━━━━━━━━━━━
        FANS  ◆  POWER  ◆  LIGHTING  ◆  TELEMETRY
        VERIFIED RELEASE  ·  v0.9.4  ·  x86_64
```

Retain the final newline. This fixture deliberately contains the repository's current synchronized `0.9.4` source version; Task 2 makes future version bumps update it.

- [ ] **Step 2: Strengthen the banner fixture test**

Replace the existing `banner snapshot and color behavior follow terminal capabilities` test in `packaging/installer/installer.test.mjs` with:

```javascript
test('banner snapshot and color behavior follow terminal capabilities', () => {
  const banner = fs.readFileSync(path.join(root, 'packaging/installer/banner.txt'), 'utf8');
  const lines = banner.trimEnd().split('\n');
  assert.equal(lines.length, 10);
  assert.equal(lines[0], '    ██╗   ██╗   ███████╗ █████╗ ███╗   ██╗ ██████╗');
  assert.equal(lines[5], '       ╚═╝      ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝');
  assert.equal(lines[6], '');
  assert.equal(lines[7], '    ━━━ RAZER BLADE CONTROL // INSTALLER ━━━━━━━━━━━');
  assert.equal(lines[8], '        FANS  ◆  POWER  ◆  LIGHTING  ◆  TELEMETRY');

  const bannerVersion = lines[9].match(
    /^        VERIFIED RELEASE  ·  v(\d+\.\d+\.\d+)  ·  x86_64$/
  );
  assert.ok(bannerVersion, 'banner metadata must contain a semantic version and x86_64');
  assert.equal(bannerVersion[1], version);
  for (const line of lines) {
    assert.ok([...line].length <= 72, `banner line is wider than 72 columns: ${line}`);
  }

  const noColor = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
    tty: '1'
  });
  const noColorResult = noColor.run();
  assert.equal(noColorResult.status, 0, noColorResult.stdout + noColorResult.stderr);
  assert.ok(noColorResult.stdout.startsWith(banner));
  assert.equal(noColorResult.stdout.indexOf(banner), noColorResult.stdout.lastIndexOf(banner));
  assert.doesNotMatch(noColorResult.stdout, /\u001b\[/);
  noColor.cleanup();

  const color = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
    tty: '1',
    noColor: false
  });
  const colorResult = color.run();
  assert.equal(colorResult.status, 0, colorResult.stdout + colorResult.stderr);
  assert.match(
    colorResult.stdout,
    /\u001b\[1;92m    ██╗   ██╗   \u001b\[1;97m███████╗/
  );
  assert.match(
    colorResult.stdout,
    /\u001b\[1;96m    ━━━ RAZER BLADE CONTROL \/\/ INSTALLER ━━━━━━━━━━━/
  );
  assert.match(
    colorResult.stdout,
    new RegExp(`\\u001b\\[37m        VERIFIED RELEASE  ·  v${version.replaceAll('.', '\\.')}  ·  x86_64`)
  );
  const strippedColor = colorResult.stdout.replace(/\u001b\[[0-9;]*m/g, '');
  assert.ok(strippedColor.startsWith(banner));
  assert.equal(strippedColor.indexOf(banner), strippedColor.lastIndexOf(banner));
  assert.doesNotMatch(strippedColor, /\u001b/);
  color.cleanup();

  const nonInteractive = makeFixture({
    osRelease: 'ID=ubuntu\nVERSION_ID="24.04"\nVERSION_CODENAME=noble\n',
    tty: '0',
    noColor: false
  });
  const nonInteractiveResult = nonInteractive.run();
  assert.equal(nonInteractiveResult.status, 0);
  assert.doesNotMatch(nonInteractiveResult.stdout, /RAZER BLADE CONTROL|███████╗/);
  assert.doesNotMatch(nonInteractiveResult.stdout, /\u001b\[/);
  nonInteractive.cleanup();
});
```

- [ ] **Step 3: Run the focused test and verify the old renderer fails**

Run:

```bash
node --test --test-name-pattern='banner snapshot' packaging/installer/installer.test.mjs
```

Expected: FAIL at `noColorResult.stdout.startsWith(banner)` because `install.sh` still renders the old five-line bordered panel.

- [ ] **Step 4: Define the four banner color roles**

Replace `configure_output()` in `install.sh` with:

```bash
configure_output() {
  OUTPUT_TTY=0
  if [[ ${FANG_INSTALLER_TESTING:-} == 1 ]]; then
    OUTPUT_TTY=${FANG_TEST_TTY:-0}
  elif [[ -t 1 ]]; then
    OUTPUT_TTY=1
  fi

  COLOR_SUCCESS=
  COLOR_CURRENT=
  COLOR_WARNING=
  COLOR_ERROR=
  COLOR_BANNER_MARK=
  COLOR_BANNER_WORDMARK=
  COLOR_BANNER_HUD=
  COLOR_BANNER_METADATA=
  COLOR_RESET=
  if [[ $OUTPUT_TTY == 1 && -z ${NO_COLOR+x} ]]; then
    COLOR_SUCCESS=$'\033[32m'
    COLOR_CURRENT=$'\033[36m'
    COLOR_WARNING=$'\033[33m'
    COLOR_ERROR=$'\033[31m'
    COLOR_BANNER_MARK=$'\033[1;92m'
    COLOR_BANNER_WORDMARK=$'\033[1;97m'
    COLOR_BANNER_HUD=$'\033[1;96m'
    COLOR_BANNER_METADATA=$'\033[37m'
    COLOR_RESET=$'\033[0m'
  fi
}
```

The first four progress assignments are intentionally unchanged.

- [ ] **Step 5: Render the mark and wordmark as adjacent color segments**

Replace `print_banner()` in `install.sh` with:

```bash
print_banner() {
  [[ $OUTPUT_TTY == 1 ]] || return 0
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '    ██╗   ██╗   ' "$COLOR_BANNER_WORDMARK" \
    '███████╗ █████╗ ███╗   ██╗ ██████╗' "$COLOR_RESET"
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '    ██║   ██║   ' "$COLOR_BANNER_WORDMARK" \
    '██╔════╝██╔══██╗████╗  ██║██╔════╝' "$COLOR_RESET"
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '    ╚██╗ ██╔╝   ' "$COLOR_BANNER_WORDMARK" \
    '█████╗  ███████║██╔██╗ ██║██║  ███╗' "$COLOR_RESET"
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '     ╚████╔╝    ' "$COLOR_BANNER_WORDMARK" \
    '██╔══╝  ██╔══██║██║╚██╗██║██║   ██║' "$COLOR_RESET"
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '      ╚██╔╝     ' "$COLOR_BANNER_WORDMARK" \
    '██║     ██║  ██║██║ ╚████║╚██████╔╝' "$COLOR_RESET"
  printf '%b%s%b%s%b\n' "$COLOR_BANNER_MARK" \
    '       ╚═╝      ' "$COLOR_BANNER_WORDMARK" \
    '╚═╝     ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝' "$COLOR_RESET"
  printf '\n'
  printf '%b%s%b\n' "$COLOR_BANNER_HUD" \
    '    ━━━ RAZER BLADE CONTROL // INSTALLER ━━━━━━━━━━━' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER_HUD" \
    '        FANS  ◆  POWER  ◆  LIGHTING  ◆  TELEMETRY' "$COLOR_RESET"
  printf '%b%s%b\n' "$COLOR_BANNER_METADATA" \
    "        VERIFIED RELEASE  ·  v${VERSION}  ·  x86_64" "$COLOR_RESET"
}
```

Do not introduce a helper process, cursor escape, delay, or second version constant.

- [ ] **Step 6: Run the focused test and verify all banner modes pass**

Run:

```bash
node --test --test-name-pattern='banner snapshot' packaging/installer/installer.test.mjs
```

Expected: PASS. The interactive no-color output begins with the fixture, colored output strips to the same fixture with the specified role codes, and noninteractive output has neither banner text nor ANSI.

- [ ] **Step 7: Verify Bash syntax and shell lint**

Run:

```bash
bash -n install.sh
shellcheck install.sh
```

Expected: both commands exit 0 with no output.

- [ ] **Step 8: Commit the renderer and its fixture contract**

```bash
git add install.sh packaging/installer/banner.txt packaging/installer/installer.test.mjs
git commit -m "feat(installer): add neon Fang banner"
```

### Task 2: Synchronize the banner snapshot version

**Files:**
- Modify: `app/scripts/version.test.mjs:11-23,97-125`
- Modify: `app/scripts/version.mjs:82-109,151-225`

**Interfaces:**
- Consumes: the exact snapshot row `        VERIFIED RELEASE  ·  vVERSION  ·  x86_64`.
- Produces: `currentVersions()` entry named `installer banner` and `setVersion(version: string)` updates to `packaging/installer/banner.txt`.

- [ ] **Step 1: Copy the banner into isolated version fixtures**

Add the snapshot path after `app/src-tauri/tauri.conf.json` in the `files` array in `app/scripts/version.test.mjs`:

```javascript
  'app/src-tauri/tauri.conf.json',
  'packaging/installer/banner.txt',
  'packaging/rpm/fang.spec',
```

- [ ] **Step 2: Add failing setter and stale-snapshot assertions**

In `set updates both RPM versions and the next-minor upper bound`, add this assertion immediately after the two installer identity assertions:

```javascript
  assert.match(
    fs.readFileSync(path.join(dir, 'packaging/installer/banner.txt'), 'utf8'),
    /^        VERIFIED RELEASE  ·  v0\.10\.0  ·  x86_64$/m
  );
```

Add this test immediately after `check rejects stale release-installer identity`:

```javascript
test('check rejects a stale installer banner identity', () => {
  const dir = fixture();
  const banner = path.join(dir, 'packaging/installer/banner.txt');
  const stale = mutateFixture(
    fs.readFileSync(banner, 'utf8'),
    `v${fixtureVersion}`,
    'v9.8.7'
  );
  fs.writeFileSync(banner, stale);
  const result = run(dir, 'check');
  assert.notEqual(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stderr, /installer banner|synchronized/i);
  fs.rmSync(dir, { recursive: true });
});
```

- [ ] **Step 3: Run the version tests and verify both new contracts fail**

Run:

```bash
node --test app/scripts/version.test.mjs
```

Expected: FAIL because `set 0.10.0` leaves the snapshot at `v0.9.4`, and `check` does not yet reject a deliberately stale snapshot.

- [ ] **Step 4: Include the snapshot in `currentVersions()`**

In `currentVersions()` in `app/scripts/version.mjs`, read the banner next to the installer:

```javascript
  const changelog = read('CHANGELOG.md');
  const installer = read('install.sh');
  const banner = read('packaging/installer/banner.txt');
  const fangRpm = read('packaging/rpm/fang.spec');
```

Add this tuple immediately after the existing `release installer` tuple:

```javascript
    ['release installer', capture('release installer version', installer, /^readonly VERSION='([^']+)'$/m)],
    [
      'installer banner',
      capture(
        'installer banner version',
        banner,
        /^        VERIFIED RELEASE  ·  v(\d+\.\d+\.\d+)  ·  x86_64$/m
      )
    ],
    ['CHANGELOG.md', capture('latest changelog release', changelog, /^## \[(\d+\.\d+\.\d+)\]/m)]
```

This exact row pattern makes spacing, architecture, and version identity part of release synchronization.

- [ ] **Step 5: Update the snapshot during `setVersion()`**

Immediately after writing the updated `install.sh` in `setVersion()`, add:

```javascript
  text = read('packaging/installer/banner.txt');
  text = replaceRequired(
    'installer banner version',
    text,
    /^(        VERIFIED RELEASE  ·  v)\d+\.\d+\.\d+(  ·  x86_64)$/m,
    '$1' + version + '$2'
  );
  write('packaging/installer/banner.txt', text);
```

Keep the existing installer `VERSION` and `RELEASE_TAG` replacements unchanged.

- [ ] **Step 6: Run version tests and the real repository check**

Run:

```bash
node --test app/scripts/version.test.mjs
node app/scripts/version.mjs check
```

Expected: all tests PASS and the repository check prints:

```text
Fang version sync OK: 0.9.4
```

- [ ] **Step 7: Re-run the installer snapshot test after version integration**

Run:

```bash
node --test --test-name-pattern='banner snapshot' packaging/installer/installer.test.mjs
```

Expected: PASS with the banner fixture still synchronized at `0.9.4`.

- [ ] **Step 8: Commit version synchronization**

```bash
git add app/scripts/version.mjs app/scripts/version.test.mjs
git commit -m "test(release): synchronize installer banner version"
```

### Task 3: Run the complete installer and release safety gate

**Files:**
- Verify only: `install.sh`
- Verify only: `packaging/install-from-source.sh`
- Verify only: `packaging/installer/banner.txt`
- Verify only: `packaging/installer/installer.test.mjs`
- Verify only: `packaging/release/publish.sh`
- Verify only: `packaging/release/*.test.mjs`
- Verify only: `packaging/deb/verify.sh`
- Verify only: `packaging/rpm/build.sh`
- Verify only: `packaging/rpm/verify.sh`
- Verify only: `app/scripts/version.mjs`
- Verify only: `app/scripts/version.test.mjs`

**Interfaces:**
- Consumes: the completed renderer and version-sync commits from Tasks 1 and 2.
- Produces: fresh local evidence that installer, release, syntax, lint, and whitespace contracts remain green.

- [ ] **Step 1: Run the complete installer fixture suite**

Run:

```bash
node packaging/installer/installer.test.mjs
```

Expected: every installer fixture passes with zero failures, including platform, checksum, metadata, downgrade, transaction, service, group, cleanup, truncation, and banner behavior.

- [ ] **Step 2: Run version and release-contract suites**

Run:

```bash
node app/scripts/version.mjs check
node --test app/scripts/version.test.mjs
node packaging/release/release-contract.test.mjs
node packaging/release/publish.test.mjs
node packaging/deb/verify.test.mjs
node --test packaging/rpm/build-script.test.mjs packaging/rpm/metadata.test.mjs
```

Expected: version synchronization reports `0.9.4`; every Node suite passes with zero failures.

- [ ] **Step 3: Run syntax and ShellCheck gates**

Run:

```bash
bash -n install.sh
shellcheck install.sh packaging/install-from-source.sh packaging/rpm/build.sh packaging/rpm/verify.sh packaging/deb/verify.sh packaging/release/publish.sh
```

Expected: both commands exit 0 with no diagnostics.

- [ ] **Step 4: Check whitespace and the scoped diff**

Run:

```bash
git diff --check
git status --short
git diff d65062f..HEAD -- install.sh packaging/installer/banner.txt packaging/installer/installer.test.mjs app/scripts/version.mjs app/scripts/version.test.mjs
```

Expected:

- `git diff --check` exits 0 with no output.
- The scoped diff contains only the approved banner renderer, its tests and fixture, and banner-version synchronization.
- `docs/superpowers/plans/2026-07-23-beginner-friendly-readme.md` may remain untracked and is not staged or modified by this work.
- No version, tag, release asset, installer safety flow, app changelog entry, or GitHub release has changed.
