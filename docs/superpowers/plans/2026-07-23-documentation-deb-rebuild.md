# Documentation Synchronization and DEB Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Synchronize Fang's current documentation and in-app changelog with v0.9.4, then build and non-destructively validate the matching desktop and daemon DEBs in the Desktop workspace.

**Architecture:** Keep the Support screen as the source of truth for accepted USDT networks, add regression coverage around the two changelog surfaces, and preserve historical release entries while recording the warning removal in v0.9.4. Build the daemon and Tauri packages with the same commands as CI, collect the exact pair in `target/deb-dist/`, and validate metadata and archive contents without installing them.

**Tech Stack:** Svelte 5, Node.js built-in test runner, Vite, Tauri 2, Rust/Cargo, cargo-deb, Debian `dpkg-deb`, Markdown.

## Global Constraints

- Release version remains exactly `0.9.4`.
- Accepted USDT networks are BNB Smart Chain (BEP20) and Ethereum (ERC20).
- The previous transfer-warning paragraph remains absent from the Support screen.
- Preserve the v0.9.2 historical record; document removal in v0.9.4.
- Historical plans and specifications under `docs/superpowers/` remain unchanged.
- Required outputs are `Fang_0.9.4_amd64.deb` and `fangd_0.9.4-1_amd64.deb`.
- Collect the exact pair in `target/deb-dist/`.
- Do not install either package, enable `fangd`, or change system group membership.

---

## File Map

- Create `app/src/lib/changelog-content.test.js`: regression contract between
  the in-app release panel and repository changelog.
- Modify `app/src/screens/Changelog.svelte`: add condensed v0.9.3 and v0.9.4
  release entries.
- Modify `CHANGELOG.md`: record removal of the old warning in v0.9.4 without
  rewriting v0.9.2.
- Modify `app/src/lib/support-content.test.js`: require current README USDT
  network documentation.
- Modify `README.md`: add a current Support Fang section naming both accepted
  USDT networks.
- Audit without changing `CONTRIBUTING.md` and `HARDWARE_TESTING.md`: both
  already document the current immutable release and source-build paths.
- Generate ignored artifacts under `target/debian/`,
  `app/src-tauri/target/release/bundle/deb/`, and `target/deb-dist/`.

### Task 1: Synchronize the repository and in-app changelogs

**Files:**

- Create: `app/src/lib/changelog-content.test.js`
- Modify: `app/src/screens/Changelog.svelte:3-26`
- Modify: `CHANGELOG.md:7-27`

**Interfaces:**

- Consumes: the existing static `RELEASES` array and the repository's
  Keep-a-Changelog release sections.
- Produces: ordered v0.9.4, v0.9.3, and v0.9.2 panel entries plus an explicit
  v0.9.4 removal record.

- [ ] **Step 1: Add a failing changelog contract test**

Create `app/src/lib/changelog-content.test.js` with:

```js
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const panel = fs.readFileSync(path.join(root, 'app/src/screens/Changelog.svelte'), 'utf8');
const changelog = fs.readFileSync(path.join(root, 'CHANGELOG.md'), 'utf8');

test('the in-app changelog contains the latest releases in descending order', () => {
  const v094 = panel.indexOf("version: '0.9.4'");
  const v093 = panel.indexOf("version: '0.9.3'");
  const v092 = panel.indexOf("version: '0.9.2'");

  assert.ok(v094 >= 0, 'v0.9.4 must be present');
  assert.ok(v093 > v094, 'v0.9.3 must follow v0.9.4');
  assert.ok(v092 > v093, 'v0.9.2 must follow v0.9.3');
});

test('v0.9.4 records installer, USDT network, and warning-removal changes', () => {
  assert.match(panel, /release-locked one-command installer/i);
  assert.match(panel, /BNB Smart Chain \(BEP20\).*Ethereum \(ERC20\)/);
  assert.match(panel, /generic crypto-transfer warning.*were removed/i);
  assert.match(
    changelog,
    /## \[0\.9\.4\][\s\S]*?### Removed[\s\S]*?generic crypto-transfer warning/i
  );
});
```

- [ ] **Step 2: Run the focused test and verify the new contract fails**

Run:

```bash
node --test app/src/lib/changelog-content.test.js
```

Expected: FAIL because the panel has no v0.9.4 or v0.9.3 entry and
`CHANGELOG.md` has no v0.9.4 `Removed` section.

- [ ] **Step 3: Add v0.9.4 and v0.9.3 to the in-app changelog**

Insert these objects at the beginning of `RELEASES`, before v0.9.2:

```js
    {
      version: '0.9.4',
      date: '2026-07-23',
      title: 'Immutable release installer',
      groups: [
        {
          kind: 'Added',
          items: [
            'A release-locked one-command installer selects and validates the matching Fang and fangd package pair before asking for sudo.',
            'USDT donations identify BNB Smart Chain (BEP20) and Ethereum (ERC20) as the accepted networks.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'Releases publish as an immutable six-asset set containing the installer, checksum manifest, two DEBs and two RPMs.'
          ]
        },
        {
          kind: 'Removed',
          items: [
            'The previous generic crypto-transfer warning and instruction to confirm the USDT network with the creator were removed.'
          ]
        }
      ]
    },
    {
      version: '0.9.3',
      date: '2026-07-18',
      title: 'Fedora RPM support',
      groups: [
        {
          kind: 'Added',
          items: [
            'Native x86_64 RPM packages support Fedora 43 and Fedora 44.',
            'Fedora package gates cover build, installation, launch, dependencies and removal.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'GitHub releases are created only after both DEBs and both RPMs pass their release gates.'
          ]
        }
      ]
    },
```

- [ ] **Step 4: Record the removal in the repository changelog**

After the v0.9.4 `Changed` bullets and before v0.9.3, add:

```markdown
### Removed

- Removed the generic crypto-transfer warning and the instruction to confirm
  the USDT network with the Fang creator. The Support screen now states both
  accepted networks directly.
```

- [ ] **Step 5: Run the changelog and full app test suites**

Run:

```bash
node --test app/src/lib/changelog-content.test.js
npm test --prefix app
```

Expected: both commands PASS; the app suite includes the new changelog test.

- [ ] **Step 6: Build the frontend**

Run:

```bash
npm run build --prefix app
```

Expected: Vite completes successfully with no Svelte compile errors.

- [ ] **Step 7: Commit the changelog synchronization**

```bash
git add app/src/lib/changelog-content.test.js app/src/screens/Changelog.svelte CHANGELOG.md
git commit -m "docs: synchronize the v0.9.4 changelogs"
```

### Task 2: Synchronize current user-facing documentation

**Files:**

- Modify: `app/src/lib/support-content.test.js:1-18`
- Modify: `README.md:173`
- Audit: `CONTRIBUTING.md`
- Audit: `HARDWARE_TESTING.md`

**Interfaces:**

- Consumes: the network names already enforced by the Support screen test.
- Produces: a README Support Fang section with the same exact network names.

- [ ] **Step 1: Extend the support-copy regression test**

Add this declaration after the existing `source` declaration:

```js
const readme = fs.readFileSync(new URL('../../../README.md', import.meta.url), 'utf8');
```

Then append this test:

```js
test('README names both accepted USDT networks', () => {
  assert.match(
    readme,
    /USDT[\s\S]*?BNB Smart Chain \(BEP20\)[\s\S]*?Ethereum \(ERC20\)/
  );
});
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
node --test app/src/lib/support-content.test.js
```

Expected: FAIL because README does not yet describe Fang's USDT networks.

- [ ] **Step 3: Add the current Support Fang documentation**

Insert this section after `Build from source on Ubuntu/Debian` and before
`Supported hardware`:

```markdown
## Support Fang

Fang's in-app Support screen lists the creator's BTC, USDT and Solana donation
addresses. The USDT address accepts BNB Smart Chain (BEP20) and Ethereum
(ERC20).
```

- [ ] **Step 4: Run the support and full app test suites**

Run:

```bash
node --test app/src/lib/support-content.test.js
npm test --prefix app
```

Expected: both commands PASS.

- [ ] **Step 5: Audit all maintained current documentation**

Run:

```bash
rg -n -i "usdt|crypto transfers|transfer-safety|transfer safety|0\\.9\\.4|install\\.sh|install-from-source" README.md CHANGELOG.md CONTRIBUTING.md HARDWARE_TESTING.md app/src/screens/Changelog.svelte app/src/screens/Support.svelte
```

Expected:

- README and both changelogs name BEP20 and ERC20.
- `Support.svelte` contains no removed warning text.
- `CONTRIBUTING.md` still documents the immutable six-asset workflow.
- `HARDWARE_TESTING.md` uses the release `install.sh` and
  `packaging/install-from-source.sh` correctly.
- Historical v0.9.2 changelog text may still describe the warning that existed
  in that release.

- [ ] **Step 6: Check current Markdown formatting**

Run:

```bash
git diff --check
for doc in README.md CHANGELOG.md CONTRIBUTING.md HARDWARE_TESTING.md; do
  awk '/^```/{count++} END{exit count % 2}' "$doc"
done
```

Expected: no whitespace errors and every current Markdown file has balanced
fenced code blocks.

- [ ] **Step 7: Commit the README synchronization**

```bash
git add app/src/lib/support-content.test.js README.md
git commit -m "docs: document accepted USDT networks"
```

### Task 3: Verify sources and rebuild the exact v0.9.4 DEB pair

**Files:**

- Generate: `target/debian/fangd_0.9.4-1_amd64.deb`
- Generate: `app/src-tauri/target/release/bundle/deb/Fang_0.9.4_amd64.deb`
- Generate: `target/deb-dist/Fang_0.9.4_amd64.deb`
- Generate: `target/deb-dist/fangd_0.9.4-1_amd64.deb`

**Interfaces:**

- Consumes: synchronized v0.9.4 manifests, frontend source, Rust source, and
  existing local cargo-deb/system build dependencies.
- Produces: an exact, matching, non-installed Ubuntu/Debian package pair.

- [ ] **Step 1: Verify versions and relevant release contracts**

Run:

```bash
node app/scripts/version.mjs check
node --test app/scripts/version.test.mjs
node --test packaging/installer/installer.test.mjs
node --test packaging/release/release-contract.test.mjs
node --test packaging/deb/verify.test.mjs
```

Expected: version output is `Fang version sync OK: 0.9.4`; all Node test suites
PASS.

- [ ] **Step 2: Run the Rust workspace tests**

Run:

```bash
cargo test --workspace
```

Expected: all workspace tests PASS.

- [ ] **Step 3: Reinstall locked frontend dependencies**

Run:

```bash
npm ci --prefix app
```

Expected: npm installs from `app/package-lock.json` successfully and reports no
lockfile changes.

- [ ] **Step 4: Build the daemon DEB**

Run:

```bash
cargo deb -p fangd
```

Expected:
`target/debian/fangd_0.9.4-1_amd64.deb` exists.

- [ ] **Step 5: Build only the Fang desktop DEB**

Run:

```bash
npm --prefix app run tauri -- build --bundles deb
```

Expected:
`app/src-tauri/target/release/bundle/deb/Fang_0.9.4_amd64.deb` exists.

- [ ] **Step 6: Collect a clean two-package handoff**

Run:

```bash
mkdir -p target/deb-dist
find target/deb-dist -maxdepth 1 -type f -name '*.deb' -delete
cp target/debian/fangd_0.9.4-1_amd64.deb target/deb-dist/
cp app/src-tauri/target/release/bundle/deb/Fang_0.9.4_amd64.deb target/deb-dist/
test "$(find target/deb-dist -maxdepth 1 -type f -name '*.deb' | wc -l)" -eq 2
```

Expected: `target/deb-dist/` contains exactly the requested package pair.

- [ ] **Step 7: Validate package identity without installation**

Run:

```bash
test "$(dpkg-deb -f target/deb-dist/Fang_0.9.4_amd64.deb Package)" = fang
test "$(dpkg-deb -f target/deb-dist/Fang_0.9.4_amd64.deb Version)" = 0.9.4
test "$(dpkg-deb -f target/deb-dist/Fang_0.9.4_amd64.deb Architecture)" = amd64
test "$(dpkg-deb -f target/deb-dist/fangd_0.9.4-1_amd64.deb Package)" = fangd
test "$(dpkg-deb -f target/deb-dist/fangd_0.9.4-1_amd64.deb Version)" = 0.9.4-1
test "$(dpkg-deb -f target/deb-dist/fangd_0.9.4-1_amd64.deb Architecture)" = amd64
app_dependencies="$(dpkg-deb -f target/deb-dist/Fang_0.9.4_amd64.deb Depends)"
case "$app_dependencies" in
  *"fangd (>= 0.9.4)"*"fangd (<< 0.10.0)"*) ;;
  *) printf '%s\n' "$app_dependencies" >&2; exit 1 ;;
esac
```

Expected: every command exits zero and the Fang package requires the matching
v0.9 release line.

- [ ] **Step 8: Validate package contents without extraction or installation**

Run:

```bash
dpkg-deb -c target/deb-dist/Fang_0.9.4_amd64.deb | rg 'usr/bin/fang$'
dpkg-deb -c target/deb-dist/Fang_0.9.4_amd64.deb | rg 'usr/share/applications/(Fang|fang)\\.desktop$'
dpkg-deb -c target/deb-dist/fangd_0.9.4-1_amd64.deb | rg 'usr/bin/fangd$'
dpkg-deb -c target/deb-dist/fangd_0.9.4-1_amd64.deb | rg 'lib/systemd/system/fangd\\.service$'
```

Expected: each command prints exactly the matching archive entry.

- [ ] **Step 9: Run the final source and artifact verification**

Run:

```bash
npm test --prefix app
npm run build --prefix app
node app/scripts/version.mjs check
git diff --check
git status --short
sha256sum target/deb-dist/Fang_0.9.4_amd64.deb target/deb-dist/fangd_0.9.4-1_amd64.deb
```

Expected:

- tests and frontend build PASS;
- versions remain synchronized at v0.9.4;
- no whitespace errors;
- the worktree is clean because generated packages are ignored; and
- SHA-256 digests are printed for both handoff files.
