use svd_lint_core::{ir::CanonicalDevice, *};

const COMPLEX: &str = include_str!("../../../tests/fixtures/normalization/complex.svd");
const ERRORS: &str = include_str!("../../../tests/fixtures/normalization/errors.svd");

fn source(xml: &str) -> SourceFile {
    SourceFile::new("normalization.svd", xml)
}

fn valid(xml: &str) -> CanonicalDevice {
    // These tests exercise Stage 2 independently of semantic validity.
    let parsed = parse_svd(&source(xml));
    assert!(!parsed.has_errors(), "{:?}", parsed.diagnostics);
    let result = normalize(
        parsed.device.as_ref().unwrap(),
        parsed.source_map.as_ref().unwrap(),
        &NormalizeConfig::default(),
    );
    assert!(
        result.diagnostics.iter().all(|d| !d.severity.is_error()),
        "{:?}",
        result.diagnostics
    );
    result.device.unwrap()
}

fn wrap(properties: &str, registers: &str) -> String {
    format!(
        r#"<device schemaVersion="1.3"><name>D</name><version>1</version><description>d</description><addressUnitBits>8</addressUnitBits><width>32</width>{properties}<peripherals><peripheral><name>P</name><description>p</description><baseAddress>0x1000</baseAddress><registers>{registers}</registers></peripheral></peripherals></device>"#
    )
}

#[test]
fn plain_register_and_optional_properties() {
    let ir = valid(&wrap(
        "",
        "<register><name>R</name><addressOffset>4</addressOffset><size>32</size></register>",
    ));
    let r = &ir.peripherals[0].registers[0];
    assert_eq!(r.name, "P.R");
    assert_eq!(r.address, 0x1004);
    assert_eq!(r.properties.size.as_ref().unwrap().value, 32);
    assert!(r.properties.reset_value.is_none());
    assert!(r.properties.access.is_none());
}

#[test]
fn container_precedence_and_exact_property_origin() {
    let input = source(COMPLEX);
    let parsed = parse_svd(&input);
    let map = parsed.source_map.unwrap();
    let result = normalize(
        parsed.device.as_ref().unwrap(),
        &map,
        &NormalizeConfig::default(),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let ir = result.device.unwrap();
    let p = &ir.peripherals[1];
    assert_eq!(p.name, "TIMER");
    let r = p.registers.iter().find(|r| r.name == "TIMER.BASE").unwrap();
    let size = r.properties.size.as_ref().unwrap();
    assert_eq!(size.value, 16);
    let node = map.node(size.origin.declaration.unwrap()).unwrap();
    assert_eq!(input.snippet(node.range), Some("<size>16</size>"));
    assert_eq!(
        map.node(node.parent.unwrap()).unwrap().name.as_ref(),
        "peripheral"
    );
    let ctrl = p
        .registers
        .iter()
        .find(|r| r.name == "TIMER.CH[1].NEST.CTRL")
        .unwrap();
    assert_eq!(ctrl.address, 0x40a6);
    assert_eq!(ctrl.properties.size.as_ref().unwrap().value, 4);
    assert_eq!(
        ctrl.properties.access.as_ref().unwrap().value,
        svd::Access::WriteOnly
    );
    let status = p
        .registers
        .iter()
        .find(|r| r.name == "TIMER.CH[0].NEST.STATUS")
        .unwrap();
    assert_eq!(status.properties.size.as_ref().unwrap().value, 8);
    assert_eq!(ctrl.origin.arrays[0].index, "1");
    assert_eq!(ctrl.origin.arrays[0].template, "CH[%s]");
}

#[test]
fn derived_arrays_fields_enums_and_metadata() {
    let ir = valid(COMPLEX);
    let p = &ir.peripherals[1];
    let base = p.registers.iter().find(|r| r.name == "TIMER.BASE").unwrap();
    assert_eq!(base.alternate_register.as_deref(), Some("COPY"));
    assert_eq!(base.alternate_group.as_deref(), Some("VIEW"));
    assert_eq!(p.address_blocks[0].address, 0x4000);
    assert_eq!(p.interrupts[0].value, 4);
    let copy = &p.registers[0];
    assert_eq!(copy.name, "TIMER.COPY");
    assert_eq!(
        copy.properties.access.as_ref().unwrap().value,
        svd::Access::ReadOnly
    );
    // Explicit field access survives a read-only register override.
    assert_eq!(
        copy.fields[0].access.as_ref().unwrap().value,
        svd::Access::ReadWrite
    );
    assert_eq!(base.fields[1].enumerated_values[0].values.value.len(), 2);
    assert_eq!(
        base.fields[1].modified_write_values.as_ref().unwrap().value,
        svd::ModifiedWriteValues::OneToClear
    );
    assert_eq!(base.fields[2].name, "FLAGA");
    assert_eq!(base.fields[3].bit_offset, 9);
    assert_eq!(base.fields[2].enumerated_values[0].values.value.len(), 2);
    for (name, address) in [
        ("DATA2", 0x4010),
        ("DATA4", 0x4018),
        ("MORE2", 0x4030),
        ("MORE4", 0x4038),
    ] {
        let r = p
            .registers
            .iter()
            .find(|r| r.name == format!("TIMER.{name}"))
            .unwrap();
        assert_eq!(r.address, address);
        assert_eq!(r.properties.size.as_ref().unwrap().value, 8);
    }
    assert_eq!(ir.peripherals[0].registers[0].address, 0x5004);
    assert!(
        !ir.peripherals[0].registers[0]
            .origin
            .derived_from
            .is_empty()
    );
    assert_eq!(ir.peripherals[2].registers[0].address, 0x6000);
    assert_eq!(ir.peripherals[3].registers[0].address, 0x6100);
    assert_eq!(ir.peripherals[2].registers[0].fields.len(), 4);
    assert!(
        p.clusters
            .iter()
            .any(|c| c.alternate_cluster.as_deref() == Some("ALT"))
    );
}

#[test]
fn device_properties_inherit_without_invented_defaults() {
    let ir = valid(&wrap(
        "<size>16</size><access>read-only</access><resetValue>3</resetValue><resetMask>15</resetMask>",
        "<register><name>R</name><addressOffset>0</addressOffset><fields><field><name>F</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields></register>",
    ));
    let r = &ir.peripherals[0].registers[0];
    assert_eq!(r.properties.size.as_ref().unwrap().value, 16);
    assert_eq!(r.properties.reset_value.as_ref().unwrap().value, 3);
    assert_eq!(r.properties.reset_mask.as_ref().unwrap().value, 15);
    assert_eq!(
        r.fields[0].access.as_ref().unwrap().value,
        svd::Access::ReadOnly
    );
}

#[test]
fn failures_have_codes_attribute_spans_and_cycle_members() {
    let input = source(ERRORS);
    let parsed = parse_svd(&input);
    assert!(parsed.device.is_some(), "{:?}", parsed.diagnostics);
    let result = analyze_svd(&input);
    assert!(result.device.is_none());
    let codes: Vec<_> = result.diagnostics.iter().map(|d| d.code.as_str()).collect();
    for expected in [
        "NORM001", "NORM002", "NORM004", "NORM005", "NORM006", "NORM007",
    ] {
        assert!(codes.contains(&expected), "{codes:?}");
    }
    let missing = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::NormMissingReference)
        .unwrap();
    assert_eq!(
        input.snippet(missing.primary_span.unwrap()),
        Some("MISSING")
    );
    let cycles: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagnosticCode::NormCycle)
        .collect();
    assert_eq!(cycles.len(), 2);
    assert!(cycles.iter().any(|d| d.related.len() == 3));
    assert!(cycles.iter().any(|d| d.related.len() == 1));
}

#[test]
fn missing_size_is_error_and_independent_branches_continue() {
    let xml = wrap(
        "",
        "<register><name>R</name><addressOffset>0</addressOffset></register><register derivedFrom=\"UNKNOWN\"><name>X</name><addressOffset>4</addressOffset></register>",
    );
    let result = analyze_svd(&source(&xml));
    assert!(result.device.is_none());
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NormMissingProperty)
    );
}

#[test]
fn expansion_budget_is_global_and_checked_before_allocation() {
    let parsed = parse_svd(&source(COMPLEX));
    let result = normalize(
        parsed.device.as_ref().unwrap(),
        parsed.source_map.as_ref().unwrap(),
        &NormalizeConfig { max_instances: 8 },
    );
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NormExpansionLimit)
    );
    let huge = wrap(
        "<size>32</size>",
        "<register><dim>4294967295</dim><dimIncrement>4</dimIncrement><name>R%s</name><addressOffset>0</addressOffset></register>",
    );
    let result = analyze_svd(&source(&huge));
    assert_eq!(
        result.diagnostics[0].code,
        DiagnosticCode::NormExpansionLimit
    );
}

#[test]
fn ambiguous_reference_is_not_selected_by_document_order() {
    let xml = wrap(
        "<size>32</size>",
        "<register derivedFrom=\"R\"><name>X</name><addressOffset>4</addressOffset></register><register><name>R</name><addressOffset>0</addressOffset></register><register><name>R</name><addressOffset>1</addressOffset></register>",
    );
    let result = analyze_svd(&source(&xml));
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagnosticCode::NormAmbiguousReference && d.related.len() == 2)
        .unwrap();
    assert!(d.primary_span.is_some());
    assert!(result.device.is_none());
}

#[test]
fn local_child_lists_replace_base_lists() {
    let xml = wrap(
        "<size>32</size>",
        "<register><name>A</name><addressOffset>0</addressOffset><fields><field><name>BASE</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields></register><register derivedFrom=\"A\"><name>B</name><addressOffset>4</addressOffset><fields><field><name>LOCAL</name><bitOffset>3</bitOffset><bitWidth>1</bitWidth></field></fields></register>",
    );
    let ir = valid(&xml);
    assert_eq!(ir.peripherals[0].registers[1].fields.len(), 1);
    assert_eq!(ir.peripherals[0].registers[1].fields[0].name, "LOCAL");
}

#[test]
fn input_is_unchanged_and_ir_serialization_is_deterministic() {
    let parsed = parse_svd(&source(COMPLEX));
    let original = parsed.device.as_ref().unwrap().clone();
    let normalize_once = || {
        normalize(
            &original,
            parsed.source_map.as_ref().unwrap(),
            &NormalizeConfig::default(),
        )
    };
    let first = serde_json::to_string(&normalize_once().device.unwrap()).unwrap();
    let second = serde_json::to_string(&normalize_once().device.unwrap()).unwrap();
    assert_eq!(first, second);
    assert_eq!(parsed.device.unwrap(), original);
}

#[test]
fn differential_check_against_svd_parser_expand() {
    let xml = include_str!("../../../tests/fixtures/valid/derived_dim_cluster.svd");
    let canonical = valid(xml);
    let mut config = svd_parser::Config::default();
    config.validate_level = svd_parser::ValidateLevel::Disabled;
    config.expand = true;
    config.expand_properties = true;
    let expanded = svd_parser::parse_with_config(xml, &config).unwrap();
    for (ours, theirs) in canonical
        .peripherals
        .iter()
        .zip(expanded.peripherals.iter())
    {
        let registers: Vec<_> = theirs.all_registers().collect();
        assert_eq!(ours.registers.len(), registers.len());
        for (r, reference) in ours.registers.iter().zip(registers) {
            assert_eq!(
                r.name
                    .strip_prefix(&format!("{}.", ours.name))
                    .unwrap()
                    .replace('.', "_"),
                reference.name
            );
            assert_eq!(
                r.address,
                theirs.base_address + u64::from(reference.address_offset)
            );
            assert_eq!(
                r.properties.size.as_ref().map(|v| v.value),
                reference.properties.size
            );
        }
    }
}

#[test]
fn snapshot_ir() {
    let xml = include_str!("../../../tests/fixtures/valid/minimal.svd");
    insta::assert_snapshot!(serde_json::to_string_pretty(&valid(xml)).unwrap());
}

#[test]
fn snapshot_derived_arrays_ir() {
    let xml = include_str!("../../../tests/fixtures/valid/derived_dim_cluster.svd");
    insta::assert_snapshot!(serde_json::to_string_pretty(&valid(xml)).unwrap());
}

#[test]
fn snapshot_diagnostics() {
    insta::assert_snapshot!(
        serde_json::to_string_pretty(&analyze_svd(&source(ERRORS)).diagnostics).unwrap()
    );
}

#[test]
fn xml_unsupported_data_remains_reachable() {
    let xml = wrap(
        "<size>32</size><vendorExtensions><custom><name>Vendor</name></custom></vendorExtensions>",
        "<register><name>R</name><addressOffset>0</addressOffset></register>",
    );
    let input = source(&xml);
    let result = analyze_svd(&input);
    let ir = result.device.unwrap();
    let map = result.source_map.unwrap();
    let unknown = ir
        .unmodeled_nodes
        .iter()
        .map(|id| map.node(*id).unwrap())
        .find(|n| n.name.as_ref() == "vendorExtensions")
        .unwrap();
    assert_eq!(
        input.snippet(unknown.range),
        Some("<vendorExtensions><custom><name>Vendor</name></custom></vendorExtensions>")
    );
}

#[test]
fn explicit_child_usage_in_derived_parent_is_preserved() {
    let xml = wrap(
        "<size>32</size>",
        "<register><name>A</name><addressOffset>0</addressOffset></register><register derivedFrom=\"A\"><name>B</name><addressOffset>4</addressOffset><fields><field><name>LOCAL</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields></register>",
    );
    let ir = valid(&xml);
    let origin = &ir.peripherals[0].registers[1].fields[0].origin;
    assert_eq!(origin.declaration, origin.usage);
    assert!(origin.derived_from.is_empty());
}

#[test]
fn inherited_enum_value_and_usage_origins_are_exact() {
    let input = source(COMPLEX);
    let result = analyze_svd(&input);
    let map = result.source_map.unwrap();
    let ir = result.device.unwrap();
    let base = ir.peripherals[1]
        .registers
        .iter()
        .find(|r| r.name == "TIMER.BASE")
        .unwrap();
    let inherited = &base.fields[2].enumerated_values[0];
    let declaration = map
        .node(inherited.values.origin.declaration.unwrap())
        .unwrap();
    assert_eq!(declaration.name.as_ref(), "enumeratedValues");
    assert!(
        input
            .snippet(declaration.range)
            .unwrap()
            .contains("<name>MODES</name>")
    );
    let usage = inherited.usage.as_ref().unwrap();
    assert_eq!(
        input.snippet(map.node(usage.origin.declaration.unwrap()).unwrap().range),
        Some("<usage>read-write</usage>")
    );
    assert!(!inherited.values.origin.derived_from.is_empty());
}

#[test]
fn malformed_dim_index_and_name_already_rejected_by_stage_one_are_not_duplicated() {
    for registers in [
        "<register><dim>2</dim><dimIncrement>4</dimIncrement><dimIndex>Z-A</dimIndex><name>R%s</name><addressOffset>0</addressOffset></register>",
        "<register><dim>2</dim><dimIncrement>4</dimIncrement><name>R</name><addressOffset>0</addressOffset></register>",
    ] {
        let result = analyze_svd(&source(&wrap("<size>32</size>", registers)));
        assert!(result.device.is_none());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, DiagnosticCode::SvdParseError);
    }
}

#[test]
fn duplicate_indexes_and_bit_offset_overflow_are_normalization_errors() {
    let xml = wrap(
        "<size>32</size>",
        "<register><name>R</name><addressOffset>0</addressOffset><fields><field><dim>2</dim><dimIncrement>1</dimIncrement><name>F%s</name><bitOffset>4294967295</bitOffset><bitWidth>1</bitWidth></field></fields></register><register><dim>3</dim><dimIncrement>4</dimIncrement><dimIndex>A,B,B</dimIndex><name>X%s</name><addressOffset>0</addressOffset></register>",
    );
    let result = analyze_svd(&source(&xml));
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NormInvalidIndex)
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NormAddressOverflow)
    );
}

#[test]
fn cluster_container_recursion_is_reported_instead_of_expanding_forever() {
    let xml = wrap(
        "<size>32</size>",
        "<cluster><name>C</name><addressOffset>0</addressOffset><cluster derivedFrom=\"P.C\"><name>RECURSIVE</name><addressOffset>1</addressOffset></cluster></cluster>",
    );
    let result = analyze_svd(&source(&xml));
    assert!(result.device.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::NormCycle)
    );
    assert!(!result.diagnostics[0].related.is_empty());
}

#[test]
fn references_to_array_instances_resolve_lazily() {
    let xml = wrap(
        "<size>32</size>",
        r#"
      <register derivedFrom="P.C0.R1"><name>COPY</name><addressOffset>4</addressOffset></register>
      <cluster><dim>2</dim><dimIncrement>32</dimIncrement><name>C[%s]</name><addressOffset>16</addressOffset>
        <register><dim>2</dim><dimIncrement>4</dimIncrement><name>R%s</name><addressOffset>0</addressOffset><size>8</size>
          <fields><field><name>F</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields>
        </register>
      </cluster>"#,
    );
    let ir = valid(&xml);
    let r = &ir.peripherals[0].registers[0];
    assert_eq!(r.name, "P.COPY");
    assert_eq!(r.properties.size.as_ref().unwrap().value, 8);
    assert_eq!(r.fields[0].name, "F");
    assert_eq!(ir.peripherals[0].registers[4].name, "P.C[1].R1");
    assert_eq!(ir.peripherals[0].registers[4].address, 0x1034);
}

#[test]
fn reference_to_instance_with_inherited_dimension_can_point_forward() {
    let xml = wrap(
        "<size>32</size>",
        r#"
      <register derivedFrom="MORE1"><name>COPY</name><addressOffset>0</addressOffset></register>
      <register derivedFrom="BASE%s"><name>MORE%s</name><addressOffset>16</addressOffset></register>
      <register><dim>2</dim><dimIncrement>4</dimIncrement><name>BASE%s</name><addressOffset>32</addressOffset><size>8</size></register>"#,
    );
    let ir = valid(&xml);
    assert_eq!(
        ir.peripherals[0].registers[0]
            .properties
            .size
            .as_ref()
            .unwrap()
            .value,
        8
    );
    assert_eq!(ir.peripherals[0].registers[2].name, "P.MORE1");
}

#[test]
fn enum_relative_and_qualified_references_preserve_local_usage() {
    let xml = wrap(
        "<size>32</size>",
        r#"
      <register><name>R</name><addressOffset>0</addressOffset><fields>
        <field><name>A</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth>
          <enumeratedValues><name>E</name><usage>read-write</usage><enumeratedValue><name>V</name><value>0</value></enumeratedValue></enumeratedValues>
        </field>
        <field><name>B</name><bitOffset>1</bitOffset><bitWidth>1</bitWidth><enumeratedValues derivedFrom="A.E"><usage>read</usage></enumeratedValues></field>
      </fields></register>
      <register><name>S</name><addressOffset>4</addressOffset><fields>
        <field><name>A</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth><enumeratedValues derivedFrom="R.A.E"/></field>
        <field derivedFrom="P.R.A"><name>B</name><bitOffset>1</bitOffset><bitWidth>1</bitWidth></field>
        <field><name>C</name><bitOffset>2</bitOffset><bitWidth>1</bitWidth><enumeratedValues derivedFrom="P.R.A.E"/></field>
      </fields></register>"#,
    );
    let ir = valid(&xml);
    let r = &ir.peripherals[0].registers;
    assert_eq!(
        r[0].fields[1].enumerated_values[0]
            .usage
            .as_ref()
            .unwrap()
            .value,
        svd::Usage::Read
    );
    for f in &r[1].fields {
        assert_eq!(f.enumerated_values[0].values.value[0].name, "V");
        assert_eq!(
            f.enumerated_values[0].usage.as_ref().unwrap().value,
            svd::Usage::ReadWrite
        );
    }
}

#[test]
fn cycles_are_detected_for_cluster_field_and_enumerated_values() {
    for registers in [
        r#"<cluster derivedFrom="B"><name>A</name><addressOffset>0</addressOffset></cluster><cluster derivedFrom="C"><name>B</name><addressOffset>0</addressOffset></cluster><cluster derivedFrom="A"><name>C</name><addressOffset>0</addressOffset></cluster>"#,
        r#"<register><name>R</name><addressOffset>0</addressOffset><fields><field derivedFrom="B"><name>A</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field><field derivedFrom="C"><name>B</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field><field derivedFrom="A"><name>C</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields></register>"#,
        r#"<register><name>R</name><addressOffset>0</addressOffset><fields><field><name>F</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth><enumeratedValues derivedFrom="B"><name>A</name></enumeratedValues><enumeratedValues derivedFrom="C"><name>B</name></enumeratedValues><enumeratedValues derivedFrom="A"><name>C</name></enumeratedValues></field></fields></register>"#,
    ] {
        let result = analyze_svd(&source(&wrap("<size>32</size>", registers)));
        assert!(result.device.is_none());
        assert_eq!(result.diagnostics.len(), 1, "{:?}", result.diagnostics);
        assert_eq!(result.diagnostics[0].code, DiagnosticCode::NormCycle);
        assert_eq!(result.diagnostics[0].related.len(), 3);
    }
}
