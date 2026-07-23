# Beginner-Friendly README Design

## Goal

Rewrite the Fang README as a short, Reddit-friendly landing page that makes
installation obvious to people with little Linux experience while retaining
compact technical information for cautious users and contributors.

## Audience

The primary reader owns a Razer Blade, uses Linux, and may not be comfortable
with terminal commands. Secondary readers include experienced users who want to
check compatibility and safety, plus contributors who need build and
development instructions.

## Structure

Use a quick-start funnel:

1. Open with a plain-English benefit: control a Razer Blade on Linux.
2. Place the one-command installer immediately below the introduction in a
   prominent shell block:

   ```sh
   curl -fsSL https://github.com/bladeandsoulx/fang-razer-linux/releases/latest/download/install.sh | bash
   ```

3. Explain installation as “open Terminal, copy and paste, press Enter,”
   followed by the possible one-time logout requirement.
4. Show the existing screenshots and a short, scannable feature list.
5. Keep compact sections for compatibility, safety, source installation,
   development, support, credits, and license.

## Content Rules

- Prefer short sentences and common words.
- Explain unfamiliar actions at the point where users encounter them.
- Avoid duplicating feature descriptions.
- Keep claims precise: Fang targets Razer Blade laptops and does not provide
  full Razer Synapse peripheral parity.
- Preserve essential installer, hardware, thermal-safety, attribution, and
  trademark information.
- Keep advanced commands available without allowing them to dominate the page.
- Use a friendly tone without talking down to the reader.

## Validation

- Confirm the required installer command appears exactly and near the top.
- Confirm all existing local links and image paths still resolve.
- Confirm critical compatibility and thermal-safety facts remain present.
- Review the final Markdown for readable heading order, short paragraphs, and
  correct fenced code blocks.

## Out of Scope

No application code, installer behavior, screenshots, release assets, hardware
support data, or project branding will change.
