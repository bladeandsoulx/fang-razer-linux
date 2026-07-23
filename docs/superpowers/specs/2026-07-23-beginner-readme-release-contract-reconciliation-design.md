# Beginner README release-contract reconciliation

## Summary

Reconcile the approved beginner-friendly README with Fang's existing
release-documentation safety contract. The README keeps its short quick-start
structure, friendly tone, current headings, screenshots, and collapsed advanced
options. Two compact safety details return: explicit downgrade refusal and an
optional pinned-release checksum check.

The contract changes only where it still expects wording from the previous
README. Its substantive requirements for the one-command installer, review
path, integrity verification, supported platforms, non-root usage, downgrade
policy, manual packages, and source installation remain enforced.

## Context

The independently merged beginner README intentionally changed:

- `## Install in one command` to `## Install — one command`;
- a numbered three-step tutorial to a shorter “Copy. Paste. Done.” callout; and
- “Manual package installation” to the disclosure label “Install release
  packages manually.”

Those wording changes are part of the approved beginner design. The rewrite
also removed pinned v0.9.4 checksum instructions and the explicit statement
that the installer refuses downgrades. The release contract correctly exposed
that loss during the post-merge safety gate.

## Goals

- Preserve the current short and beginner-friendly README.
- Preserve every current heading, screenshot, feature summary, and disclosure.
- Restore the downgrade policy in one short clause.
- Restore optional pinned v0.9.4 integrity verification inside an existing
  collapsed section.
- Update only wording-sensitive contract assertions.
- Return the merged `main` branch to a green release safety gate.

## Non-goals

- Reverting to the previous long README.
- Restoring the old numbered installation walkthrough.
- Moving advanced verification into the top-level quick start.
- Weakening or deleting release safety requirements.
- Changing the installer, banner, packages, application, changelog, versions,
  tags, assets, or published v0.9.4 release.
- Pushing `main` or creating another GitHub release.

## Approved README changes

### Downgrade policy

Keep the existing quick-start explanation and change only its final sentence
to:

```text
The installer chooses the correct packages for your Linux system, checks them,
installs the app and background service together, upgrades an existing Fang
installation safely, and refuses downgrades.
```

This adds the protected behavior without adding a new paragraph or heading.

### Optional integrity verification

Keep the existing “Check the installer before running it” disclosure and its
three review commands. After the current explanatory sentence, add:

```text
For an extra integrity check, download the installer and checksum manifest from
the pinned v0.9.4 release:
```

Follow it with:

```bash
curl -fLO 'https://github.com/bladeandsoulx/fang-razer-linux/releases/download/v0.9.4/{install.sh,SHA256SUMS}'
grep '  install.sh$' SHA256SUMS > install.sh.sha256
sha256sum --check install.sh.sha256
```

The extra material remains hidden until a reader opens the existing advanced
option, so the main installation path stays short.

## Contract alignment

In `packaging/release/release-contract.test.mjs`:

- recognize the intentional `## Install — one command` heading;
- require the current instruction to open Terminal, paste the command, and
  press Enter;
- continue requiring the instruction to open Fang from the app menu;
- recognize the current “Install release packages manually” disclosure label;
- keep the exact one-command installer URL assertion;
- keep the inspect-first download, `less`, and `bash` assertions;
- keep pinned v0.9.4 download and `sha256sum --check` assertions;
- keep supported Ubuntu, Debian, and Fedora assertions;
- keep non-root, downgrade, source-installer, immutable-token, and hardware
  guide assertions.

No assertion becomes optional or broad enough to accept missing safety content.

## Testing

The existing release-contract test is the regression test. Before the
reconciliation, it fails consistently at the obsolete heading expectation.
After the two README additions and wording-sensitive assertion updates:

1. `node packaging/release/release-contract.test.mjs` must pass all six tests.
2. `node packaging/installer/installer.test.mjs` must still pass all 30 tests.
3. Version, publisher, DEB, RPM, Bash syntax, ShellCheck, and whitespace gates
   must remain green.
4. The scoped diff must contain only `README.md`,
   `packaging/release/release-contract.test.mjs`, and this design/plan
   documentation.

The Neon Fang feature worktree and branch remain present until the merged
`main` safety gate passes. They can then be removed according to the already
selected local-merge workflow.
