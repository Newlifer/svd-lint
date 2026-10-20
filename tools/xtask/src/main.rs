mod corpus;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use svd_lint_core::{SourceFile, analyze_svd, registry::rule_registry};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn main() {
    if let Err(e) = run() {
        eprintln!("xtask: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    std::env::set_current_dir(root())?;
    match args.first().map(String::as_str) {
Some("generate-corpus")=>generate_corpus()?,
Some("generate-docs")=>generate_docs(args.iter().any(|s|s=="--check"))?,
Some("fetch-conformance")=>fetch(args.get(1).map(String::as_str))?,
Some("test-conformance")=> {
let python=std::env::var("PYTHON").unwrap_or_else(|_|"python".into());
let status=Command::new(python).arg("tools/conformance.py").args(&args[1..]).status()?;
if !status.success(){return Err("external conformance failed".into());}
},
Some("test-corpus")=>test_corpus()?,
_=>return Err("commands: generate-docs [--check], generate-corpus, test-corpus, fetch-conformance [NAME], test-conformance [OPTIONS]".into())
}
    Ok(())
}
fn write(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, contents)
}
fn case(
    file: &str,
    input: &str,
    rule: &str,
    kind: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    write(&Path::new("tests/corpus").join(file), input)?;
    let d = corpus_diagnostics(file, input, rule == "W001");
    let codes: Vec<_> = d
        .iter()
        .map(|d| d.code().as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if kind == "negative" && !codes.contains(&rule) {
        return Err(format!("{file} fails to trigger {rule}: {codes:?}").into());
    }
    if kind != "negative" && codes.contains(&rule) {
        return Err(format!("{file} unexpectedly triggers {rule}").into());
    }
    let unexpected: &[&str] = if rule == "W003" {
        &["E019", "E020"]
    } else {
        &[]
    };
    let profile = if rule == "W001" { "strict" } else { "default" };
    let expected = json!({"expected":codes,"unexpected":unexpected,"profile":profile});
    write(
        &Path::new("tests/corpus").join(file).with_extension("json"),
        &serde_json::to_string_pretty(&expected)?,
    )?;
    Ok(json!({"rule":rule,"kind":kind,"file":file,"diagnostics":codes,"profile":profile}))
}
fn corpus_diagnostics(file: &str, input: &str, quality: bool) -> Vec<svd_lint_core::Diagnostic> {
    let mut result = analyze_svd(&SourceFile::new(file, input));
    if quality {
        if let (Some(device), Some(map)) = (&result.device, &result.source_map) {
            result
                .diagnostics
                .extend(svd_lint_core::rules::check_quality(device, map));
        }
    }
    result.diagnostics
}
fn generate_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = Vec::new();
    for (rule, valid, invalid, boundary) in corpus::semantic_cases() {
        for (kind, input) in [
            ("positive", valid),
            ("negative", invalid),
            ("boundary", boundary),
        ] {
            let file = if kind == "negative" {
                format!("invalid/{rule}/{kind}.svd")
            } else {
                format!("valid/{rule}-{kind}.svd")
            };
            cases.push(case(&file, &input, rule, kind)?);
        }
    }
    for (rule, file) in [
        ("E003", "invalid/bad_xml.svd"),
        ("E004", "invalid/wrong_root.svd"),
        ("E005", "invalid/missing_version.svd"),
        ("E007", "invalid/empty_peripherals.svd"),
        ("W002", "valid/warning_no_description.svd"),
        ("E008", "invalid/bad_number.svd"),
    ] {
        let input = fs::read_to_string(Path::new("tests/fixtures").join(file))?;
        let actual_rule = if rule == "E007" { "E005" } else { rule };
        cases.push(case(
            &format!("invalid/{actual_rule}/frontend-{}.svd", rule),
            &input,
            actual_rule,
            "negative",
        )?);
    }
    let minimal = fs::read_to_string("tests/fixtures/valid/minimal.svd")?;
    for (kind, input) in [
        ("positive", minimal.clone()),
        (
            "boundary",
            minimal.replace(
                "<description>General purpose timer</description>",
                "<description>p</description>",
            ),
        ),
    ] {
        cases.push(case(
            &format!("valid/W002-{kind}.svd"),
            &input,
            "W002",
            kind,
        )?);
    }
    let quality_good = "<device schemaVersion=\"1.3\"><name>D</name><version>1</version><description>Device</description><addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><peripherals><peripheral><name>P</name><description>Peripheral</description><baseAddress>0</baseAddress><registers><register><name>CTRL</name><description>Control register</description><addressOffset>0</addressOffset></register></registers></peripheral></peripherals></device>";
    for (kind, description) in [
        ("positive", "Control register"),
        ("negative", "CTRL"),
        ("boundary", "Mode"),
    ] {
        let input = quality_good.replace("Control register", description);
        let directory = if kind == "negative" {
            "invalid/W001"
        } else {
            "valid"
        };
        cases.push(case(
            &format!("{directory}/W001-{kind}.svd"),
            &input,
            "W001",
            kind,
        )?);
    }
    let nested = format!("{}{}", "<node>".repeat(65), "</node>".repeat(65));
    cases.push(case(
        "regression/xml-depth-limit/input.svd",
        &nested,
        "E008",
        "negative",
    )?);
    let mut registers = String::new();
    for i in 0..66 {
        let reference = if i < 65 {
            format!(" derivedFrom=\"R{}\"", i + 1)
        } else {
            String::new()
        };
        registers.push_str(&format!(
            "<register{reference}><name>R{i}</name><addressOffset>{}</addressOffset></register>",
            i * 4
        ));
    }
    let chain = format!(
        "<device schemaVersion=\"1.3\"><name>D</name><version>1</version><description>Depth budget</description><addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><peripherals><peripheral><name>P</name><description>Peripheral</description><baseAddress>0</baseAddress><registers>{registers}</registers></peripheral></peripherals></device>"
    );
    cases.push(case(
        "regression/derived-depth-limit/input.svd",
        &chain,
        "E017",
        "negative",
    )?);
    for (rule, input) in corpus::normalization_cases() {
        cases.push(case(
            &format!("invalid/{rule}/negative.svd"),
            &input,
            rule,
            "negative",
        )?);
    }
    cases.push(case(
        "regression/dim-index-whitespace/input.svd",
        &corpus::regression_whitespace(),
        "E013",
        "regression",
    )?);
    let regression = "regression/duplicate-fields-semantic-checks/input.svd";
    let input = fs::read_to_string(Path::new("tests/corpus").join(regression))?;
    cases.push(case(regression, &input, "E011", "negative")?);
    cases.push(case("valid/minimal.svd", &minimal, "E004", "positive")?);
    let derived = minimal.replace("<register>", "<register derivedFrom=\"MISSING\">");
    cases.push(case(
        "invalid/E009/missing-reference.svd",
        &derived,
        "E009",
        "negative",
    )?);
    let duplicate = minimal.replace("<version>", "<version>duplicate</version><version>");
    cases.push(case(
        "invalid/E006/duplicate-version.svd",
        &duplicate,
        "E006",
        "negative",
    )?);
    let empty = minimal.replace("<version>1.0</version>", "<version/>");
    if empty != minimal {
        cases.push(case(
            "invalid/E007/empty-version.svd",
            &empty,
            "E007",
            "negative",
        )?);
    }
    write(
        Path::new("tests/corpus/manifests/cases.json"),
        &serde_json::to_string_pretty(&json!({"version":1,"cases":cases}))?,
    )?;
    Ok(())
}
fn test_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string("tests/corpus/manifests/cases.json")?)?;
    for case in manifest["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let input = fs::read_to_string(Path::new("tests/corpus").join(file))?;
        let d = corpus_diagnostics(file, &input, case["profile"] == "strict");
        let actual: Vec<_> = d
            .iter()
            .map(|d| d.code().as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let expected: Vec<_> = case["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap())
            .collect();
        if actual != expected {
            return Err(format!("{file}: expected {expected:?}, got {actual:?}").into());
        }
    }
    println!(
        "PASS: {} corpus cases",
        manifest["cases"].as_array().unwrap().len()
    );
    Ok(())
}
fn generate_docs(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string("tests/corpus/manifests/cases.json")?)?;
    let mut rules = String::from(
        "# Rule catalogue\n\nGenerated by `cargo xtask generate-docs`; do not edit.\n\n",
    );
    for rule in rule_registry() {
        rules.push_str(&format!(
            "## {} — {}\n\nSeverity: {:?}; configurable: {}.\n\n{}\n\n",
            rule.code.as_str(),
            rule.name,
            rule.severity,
            rule.configurable,
            rule.description
        ));
        for kind in ["negative", "positive", "boundary"] {
            if let Some(c) = manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["rule"] == rule.code.as_str() && c["kind"] == kind)
            {
                let file = c["file"].as_str().unwrap();
                rules.push_str(&format!(
                    "{} example: [corpus case](../tests/corpus/{}).\n\n```xml\n{}\n```\n\n",
                    kind,
                    file,
                    fs::read_to_string(Path::new("tests/corpus").join(file))?.trim()
                ));
            }
        }
        if !["positive", "negative"].iter().all(|kind| {
            manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|case| case["rule"] == rule.code.as_str() && case["kind"] == *kind)
        }) {
            if !manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["rule"] == rule.code.as_str() && c["kind"] == "positive")
            {
                rules.push_str("Good SVD input: [minimal fixture](../tests/corpus/valid/minimal.svd). For runtime rules, this requires a readable UTF-8 file, an adequate normalization budget and, when enabled, a matching schema.\n\n");
            }
            if !manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["rule"] == rule.code.as_str() && c["kind"] == "negative")
            {
                let example = match rule.code.as_str() {
                    "E001" => "svd-lint check missing.svd",
                    "E002" => "svd-lint check non-utf8.svd",
                    "W001" => "svd-lint check short-description.svd --profile strict",
                    "E031" => "svd-lint check missing-schema-version.svd --xsd",
                    "E032" => "svd-lint check schema-invalid.svd --xsd --schema-dir schemas",
                    "E033" => "svd-lint check device.svd --xsd --schema-dir missing-schemas",
                    "E034" => "svd-lint check device.svd --xsd --schema-dir malformed-schemas",
                    _ => "svd-lint check invalid.svd",
                };
                rules.push_str(&format!(
                    "Bad runtime example:\n\n```console\n{example}\n```\n\n"
                ));
            }
            rules.push_str("Test coverage: diagnostic code round-trip, registry completeness, plus frontend/normalization, CLI I/O or schema contract tests as applicable. Runtime failures and XSD backend errors are not fabricated as semantic SVD failures.\n\n");
        }
    }
    let coverage = String::from(
        "# SVDConv coverage\n\nGenerated by `cargo xtask generate-docs`; do not edit.\n\nPer-diagnostic SVDConv mappings are currently not maintained.\n",
    );
    for (path, text) in [
        ("docs/rules.md", rules),
        ("docs/svdconv-coverage.md", coverage),
    ] {
        if check {
            if fs::read_to_string(path)? != text {
                return Err(format!("{path} is stale; run cargo xtask generate-docs").into());
            }
        } else {
            write(Path::new(path), &text)?;
        }
    }
    Ok(())
}
fn fetch(only: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let lock: Value = serde_json::from_str(&fs::read_to_string("tests/external/repos.json")?)?;
    for repo in lock.as_array().unwrap() {
        let name = repo["name"].as_str().unwrap();
        if only.is_some_and(|o| o != name) {
            continue;
        }
        let path = Path::new("tests/external/cache").join(name);
        let sha = repo["commit"].as_str().unwrap();
        if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid pinned commit".into());
        }
        if path.exists() {
            let status = repo_command(&path).args(["rev-parse", "HEAD"]).output()?;
            if status.status.success() && String::from_utf8(status.stdout)?.trim() != sha {
                return Err(format!(
                    "{} exists at a different revision; move it manually before fetching",
                    path.display()
                )
                .into());
            }
            if status.status.success() {
                continue;
            }
        }
        fs::create_dir_all(&path)?;
        for args in [
            vec!["init"],
            vec!["fetch", "--depth", "1", repo["url"].as_str().unwrap(), sha],
            vec!["checkout", "--detach", "FETCH_HEAD"],
        ] {
            if !repo_command(&path).args(args).status()?.success() {
                return Err(format!(
                    "git failed for {name}; incomplete cache is retained for inspection"
                )
                .into());
            }
        }
    }
    Ok(())
}

fn repo_command(path: &Path) -> Command {
    let mut command = Command::new("git");
    let absolute = std::env::current_dir()
        .expect("workspace current directory")
        .join(path)
        .to_string_lossy()
        .replace('\\', "/");
    command
        .arg("-c")
        .arg(format!("safe.directory={absolute}"))
        .arg("-C")
        .arg(path);
    command
}
