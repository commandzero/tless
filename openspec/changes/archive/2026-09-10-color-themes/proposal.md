## Why

Implemented color-theme contracts are mixed with user documentation and obsolete design notes in the docs bundle. Capture the implemented requirements in OpenSpec and retain historical material as archive evidence, with existing TOON requirements remaining authoritative for navigation and display extensions.

## What Changes

- Specify built-in and configured themes, runtime selection, styling precedence, terminal attributes, and Borealis source mappings.
- Reconcile TOON syntax styling with theme-specific field definitions and full-width selection backgrounds.
- Move the original theme documents and historical design notes into this change as provenance; remove their duplicate locations from docs and repair links.
- Record implementation verification, sync requirements, and archive this retrospective change. No runtime behavior changes are planned.

## Capabilities

### New Capabilities

- `color-themes`: Theme selection, configuration, semantic styling, terminal color support, and built-in palettes.

### Modified Capabilities

- `toon-rendering`: Qualify default-only styling and allow theme-specific field-definition and selection styles.

## Impact

Documentation and OpenSpec only. Existing implementation lives in src/theme.rs, src/theme/, src/config.rs, src/terminal.rs, src/lineprinter.rs, and src/screenwriter.rs. Navigation, display extensions, codec behavior, licenses, and palette source files retain their existing contracts.
