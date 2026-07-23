# Neon Fang installer banner

## Summary

Replace the release installer's small bordered heading with an original
`V FANG` terminal wordmark. The new banner uses Fang's existing green
V-shaped mark, an oversized FANG name, and a compact cyan hardware-control
HUD. It remains static, dependency-free, readable without color, and absent
from noninteractive output.

The published `v0.9.4` release is immutable and remains unchanged. This source
change lands on `main` and first ships in the next release cut from it. The
banner obtains its displayed version from the installer's release version
rather than owning a second version constant.

## Goals

- Give the one-command installer a distinctive, premium first impression.
- Arrange the Fang mark and name horizontally as `V FANG`, not vertically.
- Use an acid-green, bright-white, and cyan palette inspired by Fang's app
  icon and Razer hardware.
- Preserve the installer's existing terminal, accessibility, and safety
  contracts.
- Keep the widest banner line below 80 columns.

## Non-goals

- Animation, cursor movement, screen clearing, timing delays, or sound.
- Copying Hermes artwork, typography, caduceus, or brand language.
- Adding FIGlet, `tput`, Python, Node, or another runtime dependency.
- Changing installer progress messages or installation behavior.
- Replacing or mutating the immutable `v0.9.4` release.

## Approved composition

The no-color rendering is:

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

`v0.9.4` above illustrates the currently synchronized source version. Runtime
output substitutes the installer's `VERSION`, so the line changes with the
next normal version bump.

The leading V is Fang's mark and occupies the same six-row baseline as the
FANG wordmark. There is no outer box and no separate emblem above the name.
The open layout and horizontal HUD rule distinguish the new design from the
old five-line panel.

## Color roles

The interactive palette uses common ANSI 4-bit bright colors so it follows
the user's terminal theme:

- **Mark:** bold bright green for the leading V.
- **Wordmark:** bold bright white for FANG.
- **HUD:** bold bright cyan for the rule, separators, and feature labels.
- **Metadata:** normal white for `VERIFIED RELEASE`, the version, and
  architecture.
- **Progress:** the existing green, cyan, yellow, and red semantic colors
  remain unchanged.

Color is decorative only. No meaning is conveyed exclusively through color.
When `NO_COLOR` is present, every color role is empty and the exact approved
monochrome composition remains.

## Runtime behavior

`configure_output` continues to decide whether stdout is an interactive
terminal. It initializes the new banner color roles together with the
existing progress roles.

`print_banner` continues to return immediately for noninteractive output. On
an interactive terminal it writes the approved rows once, with no cursor
control or delays. The version row interpolates the existing release-locked
`VERSION`, and the architecture label remains `x86_64` because the installer
rejects every other architecture before installation.

The checked-in `packaging/installer/banner.txt` remains the exact no-color
snapshot for the current synchronized version. The streamed installer cannot
read repository files, so `install.sh` owns the runtime strings while the
snapshot test prevents spacing or color segmentation from changing the
visible result.

## Compatibility and accessibility

- Every visible row must contain at most 72 characters.
- The banner uses only glyph families already supported by the existing
  terminal experience: block, box-drawing, and simple geometric characters.
- There is no animation or rapid repainting.
- `NO_COLOR` produces no escape sequences.
- Piped, logged, and other non-TTY runs omit both the banner and ANSI codes.
- The installer remains safe to stream into Bash: all executable statements
  stay inside functions except the final `main "$@"` call.

## Testing

The banner fixture coverage will assert:

1. Interactive `NO_COLOR` output starts with the exact checked-in snapshot.
2. Interactive colored output contains ANSI sequences and becomes the exact
   snapshot after those sequences are removed.
3. Noninteractive output contains neither the V FANG wordmark nor ANSI
   sequences.
4. Every snapshot row is at most 72 characters.
5. The snapshot version equals the synchronized app and installer version.

Fresh verification will run:

- `node packaging/installer/installer.test.mjs`;
- the release-contract and version-synchronization tests;
- `bash -n install.sh`;
- ShellCheck for the installer and release scripts; and
- `git diff --check`.

No installer download, checksum, metadata, downgrade, privilege, package
transaction, service, or group-membership contract changes. The complete
existing installer suite must remain green.

## Research notes

Hermes' installation script uses a simple bordered panel similar to Fang's
old banner. Its richer CLI identity comes from a separate large wordmark,
hero art, and role-based palette. Fang adopts that hierarchy while retaining
its own mark, name, hardware vocabulary, and colors.

GitHub's terminal-banner engineering notes recommend semantic ANSI roles,
graceful color degradation, and avoiding automatic animation where it could
harm accessibility. This design uses a static 4-bit palette and preserves
Fang's `NO_COLOR` behavior.

References:

- <https://github.com/NousResearch/hermes-agent/blob/main/scripts/install.sh>
- <https://github.com/NousResearch/hermes-agent/blob/main/hermes_cli/banner.py>
- <https://github.blog/engineering/from-pixels-to-characters-the-engineering-behind-github-copilot-clis-animated-ascii-banner/>
