---
type: Reference
title: Vim companion palettes
description: 28 bundled Vim companion palettes and their tless role mappings.
generated: { by: codex, at: 2026-09-07T06:15:02Z }
sources:
  - id: vim-colors
    resource: https://github.com/vim/vim/tree/a96c3bc1f7f5ebb62643ae54ea8a1d2fa732aaf8/runtime/colors
---

# Vim companion palettes

All 28 top-level colorscheme files in the pinned Vim runtime have a tless
companion. Build tless with `--features colorscheme`, then select a scheme
with `tless --theme <name>`.

```sh
tless --theme desert data.json
tless --theme peachpuff data.json
```

The `vim` companion is based on Vim's `default.vim`; tless's own `default`
theme is a separate palette.

![Preview of all 28 Vim companion palettes](vim-themes.svg)

## Palette source

The palettes follow the terminal color assignments in Vim's `runtime/colors`
directory at revision `a96c3bc1f7f5ebb62643ae54ea8a1d2fa732aaf8`. The resolved
terminal values are bundled in `src/theme/vim_palettes.rs`; tless does not need
Vim or a runtime checkout when building or running.

## Palette inventory

Numbers are xterm 256-color indices. Normal lists foreground/background.
Strings and keys list their foreground. The preview uses the standard xterm
RGB values; a terminal may customize indices 0–15.

| Vim scheme | tless option | Background | Normal | String | Key |
| --- | --- | --- | --- | --- | --- |
| blue | `blue` | dark | 220/18 | 87 | 250 |
| catppuccin | `catppuccin` | dark | 189/233 | 114 | 181 |
| darkblue | `darkblue` | dark | 252/17 | 217 | 123 |
| default | `vim` | dark | 7/0 | 13 | 11 |
| delek | `delek` | light | 16/231 | 40 | 30 |
| desert | `desert` | dark | 231/236 | 217 | 120 |
| elflord | `elflord` | dark | 51/16 | 201 | 87 |
| evening | `evening` | dark | 231/236 | 217 | 51 |
| habamax | `habamax` | dark | 251/234 | 108 | 109 |
| industry | `industry` | dark | 253/16 | 51 | 201 |
| koehler | `koehler` | dark | 231/16 | 217 | 87 |
| lunaperche | `lunaperche` | dark | 251/16 | 222 | 251 |
| morning | `morning` | light | 16/254 | 201 | 30 |
| murphy | `murphy` | dark | 120/16 | 231 | 51 |
| novum | `novum` | dark | 188/233 | 157 | 153 |
| pablo | `pablo` | dark | 231/16 | 51 | 37 |
| peachpuff | `peachpuff` | light | 16/223 | 161 | 30 |
| quiet | `quiet` | dark | 253/16 | 253 | 253 |
| retrobox | `retrobox` | dark | 187/234 | 142 | 109 |
| ron | `ron` | dark | 51/16 | 51 | 51 |
| shine | `shine` | light | 16/231 | 95 | 30 |
| slate | `slate` | dark | 231/235 | 117 | 210 |
| sorbet | `sorbet` | dark | 253/233 | 179 | 113 |
| torte | `torte` | dark | 251/16 | 217 | 87 |
| unokai | `unokai` | dark | 255/235 | 185 | 81 |
| wildcharm | `wildcharm` | dark | 252/16 | 41 | 213 |
| zaibatsu | `zaibatsu` | dark | 231/16 | 227 | 123 |
| zellner | `zellner` | light | 16/231 | 201 | 21 |

## Mapping and adaptations

| tless role | Vim highlight group |
| --- | --- |
| Null | Constant |
| Boolean / number / string | Boolean / Number / String |
| Object key | Identifier |
| Empty container / delimiter / comma | Delimiter |
| Punctuation / command row / indentation | Normal |
| Array index / line number | LineNr |
| Preview text / count / ellipsis | Comment |
| Empty-row marker / truncation marker | NonText |
| Status row / path base | StatusLine |
| Information / warning / error | ModeMsg / WarningMsg / ErrorMsg |
| Focused key / array index | Visual, plus bold underline |
| Focused or paired delimiter | MatchParen, plus bold underline |
| Focused line number | CursorLineNr, plus bold underline |
| Focused truncation marker | NonText, plus bold underline |
| Search match, including previews | Search |
| Current search match | IncSearch, plus bold underline |

Search replaces the complete role/focus style. A current match inside a
collapsed preview uses ordinary Search, matching existing tless navigation.
Bold underline makes focus and the current match visible even in restrained
schemes such as Quiet. Configured color overrides apply after built-in styles,
using the existing theme keys and named 16-color values.

Unset group colors inherit Normal. When Normal itself has no explicit color,
the companion uses index 7 on 0 for dark, or 0 on 15 for light. This makes
background painting predictable. Document rows, blank rows, indentation, and
spaces after line numbers retain the scheme background. Vim companions require
a 256-color terminal with background-color erase support.

These are terminal companions. GUI RGB palettes, automatic background detection,
8/16-color fallback palettes, Vim plugins, and filetype-specific syntax overrides
are outside this mapping. None of the selected cterm groups requires italic,
undercurl, standout, or strikethrough; these attributes are omitted from the
terminal companion mappings.

## Validation

Validation commands:

```sh
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test
cargo test --all-features
okf validate docs/
```

The tests check palette colors, background resets, CLI names, config precedence,
and focus/search distinctions across every companion. Original default/Cyan
rendering tests remain in place.

## Attribution

The palette names and source assignments follow the
[Vim runtime color schemes](https://github.com/vim/vim/tree/a96c3bc1f7f5ebb62643ae54ea8a1d2fa732aaf8/runtime/colors).
The tless implementation stores only terminal palette values and its own role
mappings; it does not include Vim source code.
