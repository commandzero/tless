---
type: Guide
title: Historical TOON implementation review
description: Review record for the former vendored codec implementation.
status: draft
generated: { by: codex/gpt-6, at: 2026-09-07T05:45:35Z }
---

This historical review describes the removed codec patch. See
[published codec behavior](../toon-codec.md) for the current contract.

# Review of add-toon-support

Baseline: `e6cdef719c7319020391d6bbf838ab272ce44cf0`, compared with the working
diff and new files. The review skill's normal committed-diff workflow was adapted
because this worktree is on detached HEAD and the implementation is uncommitted.
Two independent reviewers examined Standards and Spec separately.

## Standards

No documented hard violations identified. Two actionable findings:

1. Possible Primitive Obsession in diagnostic classification. Substring searches
   over messages containing user-controlled names could classify a duplicate key
   named `UnsupportedNumber` as a numeric error. Local codec failures now carry
   typed categories; the adapter matches them before legacy syntax diagnostics.
   `diagnostic_categories_do_not_depend_on_key_contents` covers this case.
2. Terminal-test reliability. A cursor-position request could span read buffers,
   and stopping on child exit could discard queued output. The peer now retains
   incomplete requests, tests every split position, and drains output to EOF/EIO.

Both fixes are implemented and their targeted tests pass. Follow-up review found
one remaining count-error substring collision; it now also uses a typed category,
with a regression that failed before the fix. The reviewer confirmed the final
changes and reported no remaining issues in this scoped recheck.

## Spec

1. Arrays of empty objects produced invalid zero-field tables with trailing
   whitespace. They now use bare-hyphen list items, verified through the public
   adapter with an exact output and round-trip regression. The spec reviewer
   confirmed this fix.
2. Differing object key orders exposed a specification conflict.
   TOON v3 section 9.3 permits differing key order when selecting a table, then
   uses the first object's order for all rows. The pinned fixture named "uses
   field order from first object for tabular headers" requires that behavior.
   The jless output spec instead requires the same object entry order after
   round-trip. The user chose to preserve order. The encoder now uses list form
   when row key sequences differ and retains tables when they match.

The spec and fixture profile document this canonical-form exception. The pinned
fixture remains unchanged. The runner checks the explicit list-form expectation
and ordered round trips for every applicable encode case.

Initial findings: Standards 2, both addressed; Spec 2, both addressed. No registry
publication has been performed.
