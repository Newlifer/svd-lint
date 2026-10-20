# XSD compatibility spike

The isolated experiment lives in `tools/xsd-spike`, outside the normal workspace.
It tests pure Rust candidates against all 15 official schemas from the pinned
`cmsis-svd/svd-schema` commit in `tests/external/repos.json`.

| Candidate | Version | Passing schemas | Decision |
|---|---|---:|---|
| uppsala | 0.10.1 | 15/15 | admitted behind feature `xsd` |
| xmlschema + oxml | 0.0.9 + 0.0.9 | 0/15 | rejected |

Each schema must compile, accept a minimal valid document, and reject missing
required names, invalid numeric values, unknown children and missing register
offsets. Applicable revisions also test field arrays, wildcard enumeration values
and defaults. Invalid fixtures are checked against the specific schema revision:
zero width is a semantic issue, not necessarily an XSD violation.

`xmlschema` overflows the stack on the twelve 1.3 revisions; the older three
schemas also fail the fixture gate. Every schema/candidate run is a separate
process, so an abort is retained in the JSON report instead of hiding later cases.
The spike fails if the integrated candidate no longer qualifies. This gate tests
schema compatibility and the prepared fixtures; it does not establish universal
correctness of a third-party validator.

```console
cargo xtask fetch-conformance svd-schema
cargo run --manifest-path tools/xsd-spike/Cargo.toml -- tests/external/cache/svd-schema/schemas
cargo build -p svd-lint-cli --features xsd
svd-lint check device.svd --xsd --schema-dir tests/external/cache/svd-schema/schemas
```

The core boundary is `SchemaValidator`; its types contain no SARIF or XML-backend
types. Validation happens before typed SVD parsing, and an XSD error stops the
pipeline to avoid parser cascades. Missing versions/schemas and compilation errors
use the reserved XSD001–XSD004 codes. Known backend coordinates become honest
source points; missing coordinates remain absent. Backend panics that unwind are
reported as XSD004, without claiming to catch process aborts.

The provider accepts exact versioned filenames; it does not silently substitute a
different revision. `schemaVersion="1.3"` selects pinned revision 1.3.12, while
`--schema-version 1.3.10` selects exactly 1.3.10.

No schemas are bundled. Individual headers of revisions 1.3.2–1.3.12 identify
Apache-2.0; the older 1.0, 1.1, 1.2 and 1.3.1 files require separate license review
before redistribution. The repository's overall license is not treated as proof
of each file's license. `--schema-dir` provides the external schema directory.
