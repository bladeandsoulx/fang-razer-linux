# Beginner README Release-Contract Reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the approved short beginner README while restoring its explicit downgrade and pinned-checksum guidance and returning the merged release safety gate to green.

**Architecture:** `README.md` remains the user-facing source of installation guidance, with advanced integrity commands inside its existing collapsed review section. `packaging/release/release-contract.test.mjs` continues to enforce the same substantive safety requirements but recognizes the intentionally revised heading, callout, and manual-install label.

**Tech Stack:** GitHub-flavored Markdown, Node.js built-in test runner, Bash, ShellCheck

## Global Constraints

- Preserve the current short and beginner-friendly README.
- Preserve every current heading, screenshot, feature summary, and disclosure.
- Keep the exact heading `## Install — one command`.
- Keep the “Copy. Paste. Done.” quick-start presentation.
- Restore the downgrade policy in one short clause.
- Restore optional pinned v0.9.4 integrity verification inside the existing “Check the installer before running it” disclosure.
- Update only wording-sensitive release-contract assertions.
- Do not weaken or delete requirements for the one-command installer, review path, checksum verification, supported platforms, non-root usage, downgrade policy, manual packages, source installation, immutable token, or hardware guide.
- Do not change the installer, Neon Fang banner, packages, application, changelog, versions, tags, release assets, or published v0.9.4 release.
- Do not push `main` or create another GitHub release.
- Keep the Neon Fang feature worktree and branch until the merged `main` safety gate is green.

## File Structure

- `README.md` — beginner-facing installation guidance and collapsed advanced verification.
- `packaging/release/release-contract.test.mjs` — executable documentation safety contract.

---

### Task 1: Reconcile the beginner wording with the release safety contract

**Files:**
- Modify: `packaging/release/release-contract.test.mjs:147-177`
- Modify: `README.md:12-28,99-116`

**Interfaces:**
- Consumes: the current `README.md` headings, quick-start callout, advanced disclosures, and published v0.9.4 asset names.
- Produces: a README containing explicit downgrade and checksum guidance plus a six-test release-contract suite that validates the approved wording and all substantive safety content.

- [ ] **Step 1: Reproduce the current integration failure**

Run:

```bash
node packaging/release/release-contract.test.mjs
```

Expected: 5 tests pass and `documentation exposes release, review, integrity, manual, and source install paths` fails because the contract still expects `## Install in one command`.

- [ ] **Step 2: Align only wording-sensitive assertions**

In `packaging/release/release-contract.test.mjs`, replace:

```javascript
  assert.match(readme, /## Install in one command/);
  assert.match(
    readme,
    /1\. Open Terminal\..*2\. Paste the command below and press Enter\..*3\. Open Fang from your app menu/s
  );
```

with:

```javascript
  assert.match(readme, /## Install — one command/);
  assert.match(
    readme,
    /Open \*\*Terminal\*\*, paste this one line, and press \*\*Enter\*\*:/
  );
  assert.match(readme, /open \*\*Fang\*\* from your app menu/i);
```

Replace:

```javascript
  assert.match(readme, /manual package installation/i);
```

with:

```javascript
  assert.match(readme, /Install release packages manually/);
```

Leave the installer URL, inspect-first, pinned v0.9.4, checksum, platform, non-root, downgrade, source installer, immutable token, and hardware guide assertions unchanged.

- [ ] **Step 3: Verify the aligned contract exposes the missing safety content**

Run:

```bash
node packaging/release/release-contract.test.mjs
```

Expected: the documentation test still FAILS, now at the unchanged pinned v0.9.4 download assertion because the shorter README does not yet contain those integrity instructions. This confirms the contract recognizes the approved beginner wording without accepting the missing safety content.

- [ ] **Step 4: Restore the downgrade policy in the existing quick-start paragraph**

In `README.md`, replace:

```markdown
The installer chooses the correct packages for your Linux system, checks them,
installs the app and background service together, and upgrades an existing Fang
installation safely.
```

with:

```markdown
The installer chooses the correct packages for your Linux system, checks them,
installs the app and background service together, upgrades an existing Fang
installation safely, and refuses downgrades.
```

Do not add a heading, list, warning box, or additional paragraph.

- [ ] **Step 5: Restore pinned integrity verification inside the existing disclosure**

In the “Check the installer before running it” disclosure in `README.md`, keep the existing review commands and sentence:

```markdown
This lets you read the script before it asks for administrator access.
```

Immediately after that sentence, add:

````markdown

For an extra integrity check, download the installer and checksum manifest from
the pinned v0.9.4 release:

```bash
curl -fLO 'https://github.com/bladeandsoulx/fang-razer-linux/releases/download/v0.9.4/{install.sh,SHA256SUMS}'
grep '  install.sh$' SHA256SUMS > install.sh.sha256
sha256sum --check install.sh.sha256
```
````

Keep the closing `</details>` after this new block so the advanced material remains collapsed on GitHub.

- [ ] **Step 6: Verify the release documentation contract is green**

Run:

```bash
node packaging/release/release-contract.test.mjs
```

Expected: all 6 tests pass with zero failures.

- [ ] **Step 7: Verify README scope, targets, and whitespace**

Run:

```bash
test "$(rg -Fxc 'curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash' README.md)" -eq 1
perl -ne 'while (/!?\[[^\]]*\]\(([^)#]+)(?:#[^)]+)?\)/g) { print "$1\n" unless $1 =~ m{^(?:https?://|mailto:)} }' README.md |
while IFS= read -r target; do
  test -e "$target" || { echo "Missing: $target"; exit 1; }
done
git diff --check
git diff --stat
```

Expected:

- the one-command installer appears exactly once;
- every local Markdown target exists;
- whitespace validation exits 0;
- the implementation diff changes only `README.md` and `packaging/release/release-contract.test.mjs`.

- [ ] **Step 8: Commit the reconciliation**

```bash
git add README.md packaging/release/release-contract.test.mjs
git commit -m "docs: reconcile beginner README release contract"
```

### Task 2: Verify merged main and clean up the completed feature workspace

**Files:**
- Verify only: `README.md`
- Verify only: `install.sh`
- Verify only: `packaging/installer/installer.test.mjs`
- Verify only: `packaging/release/release-contract.test.mjs`
- Verify only: `packaging/release/publish.test.mjs`
- Verify only: `packaging/deb/verify.test.mjs`
- Verify only: `packaging/rpm/build-script.test.mjs`
- Verify only: `packaging/rpm/metadata.test.mjs`
- Verify only: `app/scripts/version.mjs`
- Verify only: `app/scripts/version.test.mjs`

**Interfaces:**
- Consumes: the merged Neon Fang commits and Task 1's README reconciliation commit on `main`.
- Produces: fresh evidence that merged `main` is green, followed by removal of the completed `.worktrees/neon-fang-banner` workspace and `feature/neon-fang-banner` branch.

- [ ] **Step 1: Run the complete installer fixture suite**

Run:

```bash
node packaging/installer/installer.test.mjs
```

Expected: all 30 installer fixtures pass, including the Neon Fang snapshot, colors, `NO_COLOR`, width, version, and noninteractive behavior.

- [ ] **Step 2: Run version and release suites**

Run:

```bash
node app/scripts/version.mjs check
node --test app/scripts/version.test.mjs
node packaging/release/release-contract.test.mjs
node packaging/release/publish.test.mjs
node packaging/deb/verify.test.mjs
node --test packaging/rpm/build-script.test.mjs packaging/rpm/metadata.test.mjs
```

Expected: version synchronization reports `Fang version sync OK: 0.9.4`; every Node suite passes with zero failures.

- [ ] **Step 3: Run syntax, ShellCheck, and whitespace gates**

Run:

```bash
bash -n install.sh
shellcheck install.sh packaging/install-from-source.sh packaging/rpm/build.sh packaging/rpm/verify.sh packaging/deb/verify.sh packaging/release/publish.sh
git diff --check
git status --short
```

Expected: syntax, ShellCheck, and whitespace checks exit 0 with no diagnostics; the main worktree is clean.

- [ ] **Step 4: Confirm the feature worktree is clean before removal**

Run:

```bash
git -C .worktrees/neon-fang-banner status --short
git worktree list
```

Expected: the feature worktree status is empty, and `git worktree list` shows `.worktrees/neon-fang-banner` on `feature/neon-fang-banner`.

- [ ] **Step 5: Remove the completed worktree and merged branch**

From `/home/home/Desktop/fang-Fabel5`, run:

```bash
git worktree remove /home/home/Desktop/fang-Fabel5/.worktrees/neon-fang-banner
git worktree prune
git branch -d feature/neon-fang-banner
```

Expected: Git removes the Superpowers-created worktree, prunes its registration, and reports deletion of the fully merged feature branch.

- [ ] **Step 6: Verify the final local state**

Run:

```bash
git status --short
git branch --list feature/neon-fang-banner
git worktree list
git log --oneline --decorate -8
```

Expected:

- `git status --short` is empty;
- the feature branch query returns no branch;
- only the main repository worktree remains;
- `main` contains the README reconciliation after the Neon Fang merge;
- `origin/main` is unchanged because this plan does not push.
