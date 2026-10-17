# Whitespace in dimIndex

Cause: svd-rs preserves spaces after commas in a dimIndex list; Stage 2 previously
used those strings verbatim for instance names and reference lookup.

CMSIS-SVD version: 1.3.

Affected component: svd-lint normalization (arrays and derived instance lookup).

Expected behavior: F,G, H produces RF, RG, RH, and derivedFrom="RH" resolves.
The original dimension metadata remains available in Origin.
The corpus JSON contains no diagnostics.

Discovery: pinned SVDSuite dim_handling/simple_list_register_level.svd.
Issue link: no public issue was created.

