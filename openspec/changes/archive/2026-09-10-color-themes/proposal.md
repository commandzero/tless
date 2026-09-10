## Why

Implemented color-theme contracts are mixed with user documentation and obsolete design notes in the docs bundle. Capture the implemented requirements in OpenSpec and retain historical material as archive evidence, with existing TOON requirements remaining authoritative for navigation and display extensions.

## What Changes

- Specify built-in and configured themes, runtime selection, styling precedence, terminal attributes, and Borealis source mappings.
- Reconcile TOON syntax styling with theme-specific field definitions and full-width selection backgrounds.
- Move the original theme documents and historical design notes into this change as provenance; remove their duplicate locations from docs and repair links.
- Record the already-landed runtime implementation, its verification, sync requirements, and archive this retrospective change. The archival edits add no new runtime behavior.

## Capabilities

### New Capabilities

- `color-themes`: Theme selection, configuration, semantic styling, terminal color support, and built-in palettes.

### Modified Capabilities

- `toon-rendering`: Qualify default-only styling and allow theme-specific field-definition and selection styles.

## Impact

This change documents and archives the runtime implementation in src/theme.rs, src/theme/, src/config.rs, src/terminal.rs, src/lineprinter.rs, and src/screenwriter.rs. The archival edits do not alter navigation, display extensions, codec behavior, licenses, or palette source files.
