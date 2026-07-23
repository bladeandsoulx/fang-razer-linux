# Beginner-Friendly README Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current long README with a short, Reddit-friendly page that puts the one-command installer first while retaining compact compatibility, safety, development, support, and attribution details.

**Architecture:** This is a documentation-only rewrite of `README.md`. The page will follow a quick-start funnel: benefit, installation, screenshots, features, compatibility, safety, advanced setup, development, support, and credits.

**Tech Stack:** GitHub-flavored Markdown, shell command examples, existing PNG screenshots

## Global Constraints

- The primary reader may have little Linux or terminal experience.
- The exact installer command must appear near the top:
  `curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash`
- Preserve the facts that Fang targets Razer Blade laptops, recognizes 48 models from 2015–2025, and is not a full Razer Synapse replacement for peripherals.
- Preserve critical installer compatibility, thermal-safety, attribution, license, and trademark information.
- Keep existing screenshot paths and relevant repository links valid.
- Do not change application code, installer behavior, release assets, hardware support data, screenshots, or branding.

---

### Task 1: Rewrite and validate the README

**Files:**
- Modify: `README.md`
- Reference: `docs/superpowers/specs/2026-07-23-beginner-friendly-readme-design.md`

**Interfaces:**
- Consumes: Existing screenshots under `docs/screenshots/`, repository source links, and the published GitHub installer URL.
- Produces: A standalone GitHub README for prospective users, existing users, and contributors.

- [ ] **Step 1: Replace the README with the approved quick-start structure**

Use these headings in this order:

1. `# Fang — Razer Blade Control Center for Linux`
2. `## Install — one command`
3. `## See Fang in action`
4. `## What Fang can do`
5. `## Will it work on my laptop?`
6. `## Safety`
7. `## More install options`
8. `## Development`
9. `## Support Fang`
10. `## Credits and license`

The opening must describe Fang in no more than three short lines. The install
section must tell readers to open Terminal, paste the exact command, press
Enter, and open Fang from the app menu. It must also say to run the command
without `sudo`, enter the password only when asked, and log out and back in if
the installer reports that group access was added.

Keep the existing dashboard image and four-image feature table. Reduce the
feature description to one short bullet per control. Summarize supported Linux
bases, model handling, manual/source installation, development commands,
thermal safeguards, donations, GPL-2.0 attribution, and trademark disclaimer
without repeating the same information in multiple sections.

- [ ] **Step 2: Verify the install command and critical facts**

Run:

```bash
test "$(rg -Fxc 'curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash' README.md)" -eq 1
rg -n '48|2015–2025|95 °C|87 °C|GPL-2.0|not affiliated' README.md
```

Expected: The first command exits with status 0. The second command finds the
model count/range, both thermal thresholds, license, and affiliation disclaimer.

- [ ] **Step 3: Verify all local Markdown targets**

Run:

```bash
perl -ne 'while (/!?\[[^\]]*\]\(([^)#]+)(?:#[^)]+)?\)/g) { print "$1\n" unless $1 =~ m{^(?:https?://|mailto:)} }' README.md |
while IFS= read -r target; do
  test -e "$target" || { echo "Missing: $target"; exit 1; }
done
```

Expected: The command exits with status 0 and prints no `Missing:` lines.

- [ ] **Step 4: Review Markdown quality and scope**

Run:

```bash
git diff --check
rg -n '^#{1,6} ' README.md
git diff --stat
```

Expected: `git diff --check` exits with status 0; headings appear in the planned
order; the diff contains documentation changes only.

- [ ] **Step 5: Commit the README rewrite**

```bash
git add README.md
git commit -m "docs: simplify README for new Linux users"
```

Expected: Git creates one commit containing the `README.md` rewrite.
