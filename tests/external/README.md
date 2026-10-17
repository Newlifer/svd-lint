# External conformance

Repository commits are pinned in repos.json. The cache and reports are ignored
by Git. Ordinary workspace tests never fetch or import external repositories.

```console
cargo xtask fetch-conformance
python -m venv target/conformance-env
# Activate the environment, then:
python -m pip install -r tools/conformance-requirements.txt
python -m pip install --no-deps --no-build-isolation tests/external/cache/SVDSuite
cargo build --workspace
cargo xtask test-conformance --suite SVDSuite --limit 0
python -m pytest tests/external/cache/SVDSuite/tests -q
cargo xtask test-conformance --suite data --limit 100
cargo run --manifest-path tools/xsd-spike/Cargo.toml -- tests/external/cache/svd-schema/schemas
```

xtask uses PYTHON if set, otherwise python from PATH. fetch-conformance accepts
an optional repository name. Existing caches must match the exact pinned HEAD;
the tool does not reset unrelated revisions or modify global Git configuration.

SVDSuite comparison projects peripheral, register path, address, size, access,
fields and enums. Only generated array brackets are normalized using Origin;
arbitrary user identifiers are retained. Wildcards/defaults are expanded only
in the adapter, with a 12-bit projection budget. The original IR is unchanged.

The original inputs are always reported. Where the reference supplies an absent
register size, an additional labelled variant supplies an explicit common size
to both tools. Variant differences also participate in the failure gate.
Absent access, retention of conflicting enums, reference size adjustment and
collection inheritance policies are classified explicitly. differences.json
records individually reviewed cases; it is not a wildcard whitelist.

PASS includes agreement to reject an input. EXPECTED DIFFERENCE is a known
representation/policy difference. REFERENCE AMBIGUITY preserves competing
interpretations. SVD-LINT BUG is an unreviewed mismatch and fails the harness.
--allow-unreviewed permits exploratory reporting without passing that gate.

cmsis-svd-data is sampled round-robin by vendor; --limit 0 processes all files.
Reports contain file/parse/normalization counts, errors/warnings, code counts and
vendor statistics. Warnings do not automatically prove a vendor defect.

The workflow_dispatch conformance job uploads JSON reports and runs the reference
suite separately. PR checks and internal corpus jobs are independent of it.
Official schemas are fetched from cmsis-svd/svd-schema, not bundled: see the
per-file license review and backend results in docs/xsd-spike.md.

