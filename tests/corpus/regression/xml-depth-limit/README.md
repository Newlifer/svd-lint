# Excessive XML nesting

Cause: roxmltree's recursive tokenizer overflowed the Windows main thread stack
on 65 nested elements. Source indexing and typed cluster parsing also recurse.
CMSIS-SVD version: not applicable; this is an input resource limit.
Component: frontend, before XML parsing, SourceMap and typed parsing.
Expected behavior: reject element nesting beyond 64 with SVD006, without entering
recursive analysis. A depth boundary is a tool limit, not a CMSIS-SVD violation.
Issue: found during Stage 4 fuzz target preparation; no public issue.
