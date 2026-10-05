//! Integration cases always parse XML and normalize before checking semantics.
use svd_lint_core::{ir::*, *};

fn xml(properties: &str, blocks: &str, registers: &str) -> String {
    format!(
        r#"<device schemaVersion="1.3"><name>D</name><version>1</version><description>d</description><addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><access>read-write</access>{properties}<peripherals><peripheral><name>P</name><description>p</description><baseAddress>0x1000</baseAddress>{blocks}<registers>{registers}</registers></peripheral></peripherals></device>"#
    )
}
fn reg(name: &str, offset: u32, size: u32, extra: &str, fields: &str) -> String {
    let fields = if fields.is_empty() {
        String::new()
    } else {
        format!("<fields>{fields}</fields>")
    };
    format!(
        "<register><name>{name}</name><addressOffset>{offset}</addressOffset><size>{size}</size>{extra}{fields}</register>"
    )
}
fn field(name: &str, offset: u32, width: u32, extra: &str) -> String {
    format!(
        "<field><name>{name}</name><bitOffset>{offset}</bitOffset><bitWidth>{width}</bitWidth>{extra}</field>"
    )
}
fn one(size: u32, extra: &str, fields: &str) -> String {
    xml("", "", &reg("R", 0, size, extra, fields))
}
fn entries(items: &str, usage: &str) -> String {
    let usage = if usage.is_empty() {
        String::new()
    } else {
        format!("<usage>{usage}</usage>")
    };
    format!("<enumeratedValues>{usage}{items}</enumeratedValues>")
}
fn entry(name: &str, value: &str) -> String {
    format!("<enumeratedValue><name>{name}</name><value>{value}</value></enumeratedValue>")
}
fn enum_doc(width: u32, items: &str) -> String {
    one(width, "", &field("F", 0, width, &entries(items, "")))
}
fn block(offset: u32, size: u32) -> String {
    format!(
        "<addressBlock><offset>{offset}</offset><size>{size}</size><usage>registers</usage></addressBlock>"
    )
}
fn run(input: &str) -> AnalysisResult {
    let result = analyze_svd(&SourceFile::new("semantic.svd", input));
    assert!(
        result.device.is_some(),
        "XML and normalization must succeed: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.code.as_str().starts_with("SEM")),
        "{:?}",
        result.diagnostics
    );
    result
}
fn clean(input: &str) {
    let result = run(input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}
fn finding(input: &str, code: &str, severity: Severity) {
    let result = run(input);
    let findings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .collect();
    assert!(
        !findings.is_empty(),
        "missing {code}: {:?}",
        result.diagnostics
    );
    assert!(
        findings
            .iter()
            .all(|d| d.severity == severity && d.primary_span.is_some()),
        "{findings:?}"
    );
}
macro_rules! cases {
    ($pass:ident, $fail:ident, $boundary:ident, $code:literal, $severity:expr, $valid:expr, $invalid:expr, $edge:expr) => {
        #[test]
        fn $pass() {
            clean(&$valid);
        }
        #[test]
        fn $fail() {
            finding(&$invalid, $code, $severity);
        }
        #[test]
        fn $boundary() {
            clean(&$edge);
        }
    };
}

cases!(
    sem001_valid,
    sem001_invalid,
    sem001_boundary,
    "SEM001",
    Severity::Error,
    one(32, "", ""),
    one(0, "", ""),
    one(u32::MAX, "", "")
);
cases!(
    sem002_valid,
    sem002_invalid,
    sem002_boundary,
    "SEM002",
    Severity::Error,
    one(8, "", &field("F", 0, 1, "")),
    one(8, "", &field("F", 0, 0, "")),
    one(u32::MAX, "", &field("F", 0, u32::MAX, ""))
);
cases!(
    sem003_valid,
    sem003_invalid,
    sem003_boundary,
    "SEM003",
    Severity::Error,
    one(8, "", &field("F", 0, 2, "")),
    one(8, "", &field("F", 7, 2, "")),
    one(8, "", &field("F", 7, 1, ""))
);
cases!(
    sem004_valid,
    sem004_invalid,
    sem004_boundary,
    "SEM004",
    Severity::Warning,
    one(8, "", &(field("A", 0, 1, "") + &field("B", 1, 1, ""))),
    one(8, "", &(field("A", 0, 4, "") + &field("B", 3, 2, ""))),
    one(8, "", &(field("A", 0, 4, "") + &field("B", 4, 4, "")))
);
cases!(
    sem005_valid,
    sem005_invalid,
    sem005_boundary,
    "SEM005",
    Severity::Error,
    one(8, "<resetValue>255</resetValue>", ""),
    one(8, "<resetValue>256</resetValue>", ""),
    one(64, "<resetValue>0xffffffffffffffff</resetValue>", "")
);
cases!(
    sem006_valid,
    sem006_invalid,
    sem006_boundary,
    "SEM006",
    Severity::Error,
    one(8, "<resetMask>255</resetMask>", ""),
    one(8, "<resetMask>256</resetMask>", ""),
    one(64, "<resetMask>0xffffffffffffffff</resetMask>", "")
);
cases!(
    sem007_valid,
    sem007_invalid,
    sem007_boundary,
    "SEM007",
    Severity::Warning,
    one(8, "<resetValue>1</resetValue><resetMask>1</resetMask>", ""),
    one(8, "<resetValue>2</resetValue><resetMask>1</resetMask>", ""),
    one(8, "<resetValue>0</resetValue><resetMask>0</resetMask>", "")
);
cases!(
    sem008_valid,
    sem008_invalid,
    sem008_boundary,
    "SEM008",
    Severity::Error,
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "") + &reg("B", 2, 16, "", ""))
    ),
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "") + &reg("B", 1, 16, "", ""))
    ),
    xml("", "", &(reg("A", 0, 9, "", "") + &reg("B", 2, 1, "", "")))
);
cases!(
    sem009_valid,
    sem009_invalid,
    sem009_boundary,
    "SEM009",
    Severity::Error,
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "")
            + &reg("B", 0, 16, "<alternateRegister>A</alternateRegister>", ""))
    ),
    one(16, "<alternateRegister>MISSING</alternateRegister>", ""),
    xml(
        "",
        "",
        &(reg("A", 0, 16, "", "")
            + &reg("B", 0, 16, "<alternateRegister>A</alternateRegister>", "")
            + &reg("C", 0, 16, "<alternateRegister>B</alternateRegister>", ""))
    )
);
cases!(
    sem010_valid,
    sem010_invalid,
    sem010_boundary,
    "SEM010",
    Severity::Error,
    xml("", &block(0, 8), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 8), &reg("R", 8, 8, "", "")),
    xml("", &block(0, 8), &reg("R", 7, 8, "", ""))
);
cases!(
    sem011_valid,
    sem011_invalid,
    sem011_boundary,
    "SEM011",
    Severity::Error,
    xml("", &block(0, 4), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 0), &reg("R", 0, 32, "", "")),
    xml("", &block(0, 1), &reg("R", 0, 8, "", "")).replace(
        "<baseAddress>0x1000</baseAddress>",
        "<baseAddress>0xffffffffffffffff</baseAddress>"
    )
);
cases!(
    sem012_valid,
    sem012_invalid,
    sem012_boundary,
    "SEM012",
    Severity::Error,
    enum_doc(2, &entry("A", "3")),
    enum_doc(2, &entry("A", "4")),
    enum_doc(64, &entry("A", "0xffffffffffffffff"))
);
cases!(
    sem013_valid,
    sem013_invalid,
    sem013_boundary,
    "SEM013",
    Severity::Error,
    enum_doc(2, &(entry("A", "0") + &entry("B", "1"))),
    enum_doc(2, &(entry("A", "0") + &entry("A", "1"))),
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            &(entries(&entry("A", "0"), "read") + &entries(&entry("B", "0"), "write"))
        )
    )
);
cases!(
    sem014_valid,
    sem014_invalid,
    sem014_boundary,
    "SEM014",
    Severity::Error,
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><isDefault>true</isDefault></enumeratedValue>"
    ),
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><isDefault>true</isDefault></enumeratedValue><enumeratedValue><name>E</name><isDefault>true</isDefault></enumeratedValue>"
    ),
    enum_doc(
        2,
        "<enumeratedValue><name>D</name><value>255</value><isDefault>true</isDefault></enumeratedValue>"
    )
);
cases!(
    sem015_valid,
    sem015_invalid,
    sem015_boundary,
    "SEM015",
    Severity::Error,
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            "<writeConstraint><range><minimum>0</minimum><maximum>3</maximum></range></writeConstraint>"
        )
    ),
    one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            "<writeConstraint><range><minimum>3</minimum><maximum>2</maximum></range></writeConstraint>"
        )
    ),
    one(
        64,
        "",
        &field(
            "F",
            0,
            64,
            "<writeConstraint><range><minimum>0</minimum><maximum>0xffffffffffffffff</maximum></range></writeConstraint>"
        )
    )
);
cases!(
    sem016_valid,
    sem016_invalid,
    sem016_boundary,
    "SEM016",
    Severity::Warning,
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>read-only</access><readAction>clear</readAction>"
        )
    ),
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>write-only</access><readAction>clear</readAction>"
        )
    ),
    one(
        8,
        "",
        &field(
            "F",
            0,
            1,
            "<access>read-writeOnce</access><readAction>clear</readAction><modifiedWriteValues>oneToClear</modifiedWriteValues>"
        )
    )
);

#[test]
fn invalid_fields_do_not_create_overlap_cascades() {
    let result = run(&one(
        8,
        "",
        &(field("VALID", 0, 8, "") + &field("ZERO", 0, 0, "") + &field("OUT", 7, 2, "")),
    ));
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::SemFieldOverlap)
    );
}

#[test]
fn reset_values_above_64_bits_and_absence_are_safe() {
    clean(&one(
        128,
        "<resetValue>0xffffffffffffffff</resetValue><resetMask>0xffffffffffffffff</resetMask>",
        "",
    ));
    clean(&one(8, "<resetMask>0</resetMask>", ""));
    clean(&one(8, "<resetValue>0</resetValue>", ""));
    clean(&one(8, "", ""));
}

#[test]
fn memory_units_are_not_assumed_to_be_bytes() {
    let input = xml(
        "",
        &block(0, 2),
        &(reg("A", 0, 16, "", "") + &reg("B", 1, 16, "", "")),
    )
    .replace(
        "<addressUnitBits>8</addressUnitBits>",
        "<addressUnitBits>16</addressUnitBits>",
    );
    clean(&input);
    finding(
        &input.replace(
            "<addressUnitBits>16</addressUnitBits>",
            "<addressUnitBits>8</addressUnitBits>",
        ),
        "SEM008",
        Severity::Error,
    );
}

#[test]
fn same_address_is_uncertain_and_different_peripherals_can_alias() {
    finding(
        &xml("", "", &(reg("A", 0, 8, "", "") + &reg("B", 0, 8, "", ""))),
        "SEM008",
        Severity::Warning,
    );
    let input = one(8, "", "").replace("</peripherals>", "<peripheral derivedFrom=\"P\"><name>ALIAS</name><baseAddress>0x1000</baseAddress></peripheral></peripherals>");
    clean(&input);
}

#[test]
fn alternate_groups_and_clusters_allow_explicit_views() {
    clean(&xml(
        "",
        "",
        &(reg("A", 0, 8, "<alternateGroup>VIEW</alternateGroup>", "")
            + &reg("B", 0, 8, "<alternateGroup>VIEW</alternateGroup>", "")),
    ));
    let clusters = format!(
        "<cluster><name>A</name><addressOffset>0</addressOffset>{}</cluster><cluster><name>B</name><alternateCluster>A</alternateCluster><addressOffset>0</addressOffset>{}</cluster>",
        reg("R", 0, 32, "", ""),
        reg("S", 1, 16, "", "")
    );
    clean(&xml("", "", &clusters));
}

#[test]
fn invalid_alternates_are_not_resolved_like_derived_from() {
    for reference in ["P.A", "R", "B"] {
        let input = xml(
            "",
            "",
            &(reg("A", 0, 8, "", "")
                + &reg(
                    "R",
                    0,
                    8,
                    &format!("<alternateRegister>{reference}</alternateRegister>"),
                    "",
                )
                + &reg("B", 0, 8, "", "")),
        );
        finding(&input, "SEM009", Severity::Error);
    }
    finding(
        &xml(
            "",
            "",
            &(reg("A", 0, 8, "", "")
                + &reg("B", 1, 8, "<alternateRegister>A</alternateRegister>", "")),
        ),
        "SEM009",
        Severity::Error,
    );
}

#[test]
fn address_blocks_remain_disjoint_and_invalid_blocks_do_not_cascade() {
    let blocks = block(0, 1) + &block(4, 1);
    clean(&xml("", &blocks, &reg("R", 4, 8, "", "")));
    finding(
        &xml("", &blocks, &reg("R", 2, 8, "", "")),
        "SEM010",
        Severity::Error,
    );
    let result = run(&xml(
        "",
        &block(0, 0),
        &(reg("A", 0, 8, "", "") + &reg("B", 1, 8, "", "")),
    ));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        DiagnosticCode::SemInvalidAddressBlock
    );
    clean(&one(8, "", "").replace("<registers>", "<addressBlock><offset>0</offset><size>1</size><usage>buffer</usage></addressBlock><registers>"));
}

#[test]
fn array_containment_and_conflicts_keep_distinct_physical_addresses() {
    let array = "<register><dim>4</dim><dimIncrement>1</dimIncrement><name>R%s</name><addressOffset>0</addressOffset><size>8</size></register>";
    let result = run(&xml("", &block(0, 2), array));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::SemRegisterOutsideBlock)
            .count(),
        2
    );
    let result = run(&xml(
        "",
        "",
        &array.replace("<size>8</size>", "<size>16</size>"),
    ));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::SemRegisterOverlap)
            .count(),
        3
    );
}

#[test]
fn large_arrays_deduplicate_declaration_errors_and_degenerate_ranges() {
    let array = "<register><dim>5000</dim><dimIncrement>1</dimIncrement><name>R%s</name><addressOffset>0</addressOffset><size>8</size></register>";
    let result = run(&xml("<resetValue>256</resetValue>", "", array));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        DiagnosticCode::SemResetValueOutOfBounds
    );
    let result = run(&xml(
        "",
        "",
        &array.replace(
            "<dimIncrement>1</dimIncrement>",
            "<dimIncrement>0</dimIncrement>",
        ),
    ));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        DiagnosticCode::SemRegisterOverlap
    );
}

#[test]
fn wildcard_enum_values_and_defaults_are_not_numeric_duplicates() {
    clean(&enum_doc(
        2,
        &(entry("PATTERN", "#1x") + &entry("EXACT", "2")),
    ));
    clean(&enum_doc(
        2,
        &(entry("PATTERN", "0b0x") + &entry("EXACT", "0")),
    ));
    finding(
        &enum_doc(2, &entry("PATTERN", "#1xx")),
        "SEM012",
        Severity::Error,
    );
    clean(&enum_doc(
        128,
        &(entry("PATTERN", &format!("#{}", "x".repeat(128))) + &entry("EXACT", "0")),
    ));
    let input = enum_doc(2, &(entry("PATTERN", "#1x") + &entry("EXACT", "2")))
        .replace("<enumeratedValues>", "<enumeratedValues><name>E</name>")
        .replace("</fields>", "<field><name>DERIVED</name><bitOffset>0</bitOffset><bitWidth>2</bitWidth><enumeratedValues derivedFrom=\"F.E\"/></field></fields>");
    let result = run(&input);
    assert!(!result.diagnostics.iter().any(|d| matches!(
        d.code,
        DiagnosticCode::SemDuplicateEnumEntry | DiagnosticCode::SemEnumValueOutOfBounds
    )));
}

#[test]
fn enum_aliases_warn_and_usage_sets_are_independent() {
    finding(
        &enum_doc(2, &(entry("A", "0") + &entry("B", "0"))),
        "SEM013",
        Severity::Warning,
    );
    finding(
        &enum_doc(2, &(entry("A", "0") + &entry("A", "0"))),
        "SEM013",
        Severity::Warning,
    );
}

#[test]
fn write_constraints_check_ranges_and_write_usage() {
    finding(
        &one(
            2,
            "",
            &field(
                "F",
                0,
                2,
                "<writeConstraint><range><minimum>0</minimum><maximum>4</maximum></range></writeConstraint>",
            ),
        ),
        "SEM015",
        Severity::Error,
    );
    let constraint =
        "<writeConstraint><useEnumeratedValues>true</useEnumeratedValues></writeConstraint>";
    finding(
        &one(2, "", &field("F", 0, 2, constraint)),
        "SEM015",
        Severity::Error,
    );
    finding(
        &one(
            2,
            "",
            &field(
                "F",
                0,
                2,
                &(constraint.to_owned() + &entries(&entry("A", "0"), "read")),
            ),
        ),
        "SEM015",
        Severity::Error,
    );
    clean(&one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            &(constraint.to_owned() + &entries(&entry("A", "0"), "write")),
        ),
    ));
    clean(&one(
        2,
        "",
        &field(
            "F",
            0,
            2,
            "<writeConstraint><useEnumeratedValues>false</useEnumeratedValues></writeConstraint>",
        ),
    ));
    finding(
        &one(
            2,
            "<writeConstraint><range><minimum>3</minimum><maximum>2</maximum></range></writeConstraint>",
            "",
        ),
        "SEM015",
        Severity::Error,
    );
}

#[test]
fn effective_field_access_overrides_register_and_once_access_is_legal() {
    clean(&one(
        8,
        "<access>read-only</access>",
        &field(
            "F",
            0,
            1,
            "<access>writeOnce</access><modifiedWriteValues>oneToClear</modifiedWriteValues>",
        ),
    ));
    finding(
        &one(
            8,
            "<access>read-only</access><modifiedWriteValues>oneToClear</modifiedWriteValues>",
            "",
        ),
        "SEM016",
        Severity::Warning,
    );
    finding(
        &one(
            8,
            "<access>writeOnce</access><readAction>clear</readAction>",
            "",
        ),
        "SEM016",
        Severity::Warning,
    );
    clean(&one(
        8,
        "<access>read-only</access><modifiedWriteValues>modify</modifiedWriteValues>",
        "",
    ));
}

#[test]
fn inherited_property_positions_and_conflict_related_spans_are_honest() {
    let input = xml("<resetValue>256</resetValue>", "", &reg("R", 0, 8, "", ""));
    let source = SourceFile::new("positions.svd", &input);
    let result = analyze_svd(&source);
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::SemResetValueOutOfBounds)
        .unwrap();
    assert_eq!(
        source.snippet(d.primary_span.unwrap()),
        Some("<resetValue>256</resetValue>")
    );
    assert!(!d.related.is_empty());
    let input = one(8, "", &(field("A", 0, 4, "") + &field("B", 3, 2, "")));
    let source = SourceFile::new("positions.svd", &input);
    let result = analyze_svd(&source);
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::SemFieldOverlap)
        .unwrap();
    assert_eq!(
        source.snippet(d.primary_span.unwrap()),
        Some("<name>B</name>")
    );
    assert!(
        d.related
            .iter()
            .any(|r| source.snippet(r.span) == Some("<name>A</name>"))
    );
}

#[test]
fn semantic_analysis_is_repeatable_and_does_not_mutate_ir() {
    let input = SourceFile::new("immutable.svd", one(8, "<resetValue>256</resetValue>", ""));
    let parsed = parse_svd(&input);
    let map = parsed.source_map.unwrap();
    let normalized = normalize(
        parsed.device.as_ref().unwrap(),
        &map,
        &NormalizeConfig::default(),
    );
    let ir: CanonicalDevice = normalized.device.unwrap();
    let before = serde_json::to_string(&ir).unwrap();
    let first = check_semantics(&ir, &map);
    assert_eq!(first, check_semantics(&ir, &map));
    assert_eq!(before, serde_json::to_string(&ir).unwrap());
    assert!(analyze_svd(&input).device.is_some());
    assert!(analyze_svd(&input).has_errors());
}

#[test]
fn limiting_arithmetic_values_report_errors_without_panics() {
    finding(
        &one(u32::MAX, "", &field("F", u32::MAX, u32::MAX, "")),
        "SEM003",
        Severity::Error,
    );
    let input = xml("", &block(0, 2), &reg("R", 0, 8, "", "")).replace(
        "<baseAddress>0x1000</baseAddress>",
        "<baseAddress>0xffffffffffffffff</baseAddress>",
    );
    finding(&input, "SEM011", Severity::Error);
    let input = one(16, "", "").replace(
        "<baseAddress>0x1000</baseAddress>",
        "<baseAddress>0xffffffffffffffff</baseAddress>",
    );
    finding(&input, "SEM008", Severity::Error);
    finding(
        &one(8, "", "").replace(
            "<addressUnitBits>8</addressUnitBits>",
            "<addressUnitBits>0</addressUnitBits>",
        ),
        "SEM011",
        Severity::Error,
    );
}

#[test]
fn alternate_registers_resolve_within_each_array_cluster() {
    let members = reg("BASE", 0, 8, "", "")
        + &reg(
            "VIEW",
            0,
            8,
            "<alternateRegister>BASE</alternateRegister>",
            "",
        );
    clean(&xml(
        "",
        "",
        &format!(
            "<cluster><dim>2</dim><dimIncrement>4</dimIncrement><name>C[%s]</name><addressOffset>0</addressOffset>{members}</cluster>"
        ),
    ));
}

#[test]
fn alternate_cluster_views_are_transitive_and_preserve_internal_conflicts() {
    let cluster = |name: &str, alternate: &str, members: &str| {
        format!(
            "<cluster><name>{name}</name>{alternate}<addressOffset>0</addressOffset>{members}</cluster>"
        )
    };
    let a = cluster("A", "", &reg("R", 0, 16, "", ""));
    let b = cluster(
        "B",
        "<alternateCluster>A</alternateCluster>",
        &reg("S", 1, 8, "", ""),
    );
    let c = cluster(
        "C",
        "<alternateCluster>B</alternateCluster>",
        &reg("T", 0, 16, "", ""),
    );
    clean(&xml("", "", &(a + &b + &c)));

    let conflicting = cluster(
        "A",
        "",
        &(reg("R", 0, 16, "", "") + &reg("S", 1, 8, "", "")),
    );
    let view = cluster(
        "B",
        "<alternateCluster>A</alternateCluster>",
        &reg("T", 0, 16, "", ""),
    );
    let result = run(&xml("", "", &(conflicting + &view)));
    let overlaps: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == "SEM008")
        .collect();
    assert_eq!(overlaps.len(), 1, "{overlaps:?}");
    assert_eq!(overlaps[0].severity, Severity::Error);
}

#[test]
fn alternate_array_clusters_resolve_each_physical_view() {
    let cluster = |name: &str, alternate: &str, member: &str| {
        format!(
            "<cluster><dim>2</dim><dimIncrement>4</dimIncrement><name>{name}[%s]</name>{alternate}<addressOffset>0</addressOffset>{member}</cluster>"
        )
    };
    let a = cluster("A", "", &reg("R", 0, 16, "", ""));
    let b = cluster(
        "B",
        "<alternateCluster>A[%s]</alternateCluster>",
        &reg("S", 1, 8, "", ""),
    );
    clean(&xml("", "", &(a + &b)));
}
