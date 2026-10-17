//! Run explicitly with cargo test -p svd-lint-core --test performance -- --ignored --nocapture.
use svd_lint_core::*;
fn synthetic(registers: usize, fields: usize, array: bool) -> String {
    let mut body = String::new();
    for i in 0..if array { 1 } else { registers } {
        let dim = if array {
            format!("<dim>{registers}</dim><dimIncrement>4</dimIncrement>")
        } else {
            String::new()
        };
        let name = if array { "R%s".into() } else { format!("R{i}") };
        body.push_str(&format!(
            "<register>{dim}<name>{name}</name><addressOffset>{}</addressOffset><size>{}</size>",
            i * 4,
            32.max(fields)
        ));
        if fields > 0 {
            body.push_str("<fields>");
            for n in 0..fields {
                body.push_str(&format!("<field><name>F{n}</name><bitOffset>{n}</bitOffset><bitWidth>1</bitWidth></field>"));
            }
            body.push_str("</fields>");
        }
        body.push_str("</register>");
    }
    format!(
        "<device schemaVersion=\"1.3\"><name>D</name><version>1</version><description>Performance</description><addressUnitBits>8</addressUnitBits><width>32</width><peripherals><peripheral><name>P</name><description>Peripheral</description><baseAddress>0</baseAddress><registers>{body}</registers></peripheral></peripherals></device>"
    )
}
fn measure(registers: usize, fields: usize, array: bool) {
    let source = SourceFile::new("synthetic.svd", synthetic(registers, fields, array));
    let start = std::time::Instant::now();
    let result = analyze_svd_with_config(
        &source,
        &NormalizeConfig {
            max_instances: 250_000,
        },
    );
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(
        result.device.as_ref().unwrap().peripherals[0]
            .registers
            .len(),
        registers
    );
    eprintln!(
        "{registers} registers / {fields} fields per register / array={array}: {:?}",
        start.elapsed()
    );
}
#[test]
#[ignore = "explicit performance measurement"]
fn registers_1k() {
    measure(1_000, 0, false);
}
#[test]
#[ignore = "explicit performance measurement"]
fn registers_10k() {
    measure(10_000, 0, false);
}
#[test]
#[ignore = "explicit performance measurement"]
fn fields_100k() {
    measure(1, 100_000, false);
}
#[test]
#[ignore = "explicit performance measurement"]
fn large_array() {
    measure(10_000, 0, true);
}
