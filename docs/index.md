---
okf_version: "0.2"
---

# tless documentation

The complete `docs/` directory is the OKF bundle. Source code, fixtures, root
repository documents, build output, and transient reports are outside the bundle.
Validate the complete bundle with `scripts/validate-docs.sh` locally and in CI.

## Guides

1. [Contributing](contributing.md) covers standards, local checks, and pull requests.

2. [Release checklist](release-checklist.md) covers release preparation and recovery.
3. [Published TOON codec](toon-codec.md) describes conversion behavior and known limitations.
4. [TOON acceptance](toon-acceptance.md) maps automated and manual validation.
5. [TOON document view](toon-view.md) explains layout, selection, warnings, and export boundaries.
6. [Vim companion palettes](vim-themes.md) records the 28 bundled Vim palettes.

## Specifications

[Color themes](../openspec/specs/color-themes/spec.md) defines theme selection,
configuration, and built-in palettes, including Borealis.
[TOON rendering](../openspec/specs/toon-rendering/spec.md),
[navigation](../openspec/specs/toon-navigation/spec.md), and
[display extensions](../openspec/specs/toon-display-extensions/spec.md) define
current viewer behavior.

The [archived color-themes change](../openspec/changes/archive/2026-09-10-color-themes/proposal.md)
records the migration and preserves the former design notes as historical sources.
