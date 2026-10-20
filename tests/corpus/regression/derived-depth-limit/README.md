# Long derivedFrom dependency chains

Cause: dependency resolution used unbounded recursive calls, independently of
the instance budget. The case exercises a 66-register forward chain.
CMSIS-SVD version: 1.3.
Component: normalization dependency resolver.
Expected behavior: walk direct dependencies iteratively, retain provenance, and
reject dependency depth beyond 64 with E017. This resource limit complements
max_instances; it is not a normative SVD constraint.
Issue: guarded during Stage 4 resource-limit review; no public issue.
