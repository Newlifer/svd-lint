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
    let d = corpus_diagnostics(file, input, rule == "SVD007");
    let codes: Vec<_> = d
        .iter()
        .map(|d| d.code.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if kind == "negative" && !codes.contains(&rule) {
        return Err(format!("{file} fails to trigger {rule}: {codes:?}").into());
    }
    if kind != "negative" && codes.contains(&rule) {
        return Err(format!("{file} unexpectedly triggers {rule}").into());
    }
    let unexpected: &[&str] = if rule == "SEM004" {
        &["SEM002", "SEM003"]
    } else {
        &[]
    };
    let profile = if rule == "SVD007" {
        "strict"
    } else {
        "default"
    };
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
        ("XML001", "invalid/bad_xml.svd"),
        ("SVD001", "invalid/wrong_root.svd"),
        ("SVD002", "invalid/missing_version.svd"),
        ("SVD004", "invalid/empty_peripherals.svd"),
        ("SVD005", "valid/warning_no_description.svd"),
        ("SVD006", "invalid/bad_number.svd"),
    ] {
        let input = fs::read_to_string(Path::new("tests/fixtures").join(file))?;
        let actual_rule = if rule == "SVD004" { "SVD002" } else { rule };
        cases.push(case(
            &format!("invalid/{actual_rule}/frontend-{}.svd", rule),
            &input,
            actual_rule,
            "negative",
        )?);
    }
    let minimal = fs::read_to_string("tests/fixtures/valid/minimal.svd")?;
    let quality_good = "<device schemaVersion=\"1.3\"><name>D</name><version>1</version><description>Device</description><addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><peripherals><peripheral><name>P</name><description>Peripheral</description><baseAddress>0</baseAddress><registers><register><name>CTRL</name><description>Control register</description><addressOffset>0</addressOffset></register></registers></peripheral></peripherals></device>";
    for (kind, description) in [
        ("positive", "Control register"),
        ("negative", "CTRL"),
        ("boundary", "Mode"),
    ] {
        let input = quality_good.replace("Control register", description);
        let directory = if kind == "negative" {
            "invalid/SVD007"
        } else {
            "valid"
        };
        cases.push(case(
            &format!("{directory}/SVD007-{kind}.svd"),
            &input,
            "SVD007",
            kind,
        )?);
    }
    let nested = format!("{}{}", "<node>".repeat(65), "</node>".repeat(65));
    cases.push(case(
        "regression/xml-depth-limit/input.svd",
        &nested,
        "SVD006",
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
        "NORM009",
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
        "NORM005",
        "regression",
    )?);
    let regression = "regression/duplicate-fields-semantic-checks/input.svd";
    let input = fs::read_to_string(Path::new("tests/corpus").join(regression))?;
    cases.push(case(regression, &input, "NORM003", "negative")?);
    cases.push(case("valid/minimal.svd", &minimal, "SVD001", "positive")?);
    let derived = minimal.replace("<register>", "<register derivedFrom=\"MISSING\">");
    cases.push(case(
        "invalid/NORM001/missing-reference.svd",
        &derived,
        "NORM001",
        "negative",
    )?);
    let duplicate = minimal.replace("<version>", "<version>duplicate</version><version>");
    cases.push(case(
        "invalid/SVD003/duplicate-version.svd",
        &duplicate,
        "SVD003",
        "negative",
    )?);
    let empty = minimal.replace("<version>1.0</version>", "<version/>");
    if empty != minimal {
        cases.push(case(
            "invalid/SVD004/empty-version.svd",
            &empty,
            "SVD004",
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
            .map(|d| d.code.as_str())
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
        rules.push_str(&format!("## {} — {}\n\nSeverity: {:?}; category: {:?}; source: {:?}; configurable: {}.\n\n{}\n\n",rule.code,rule.name,rule.default_severity,rule.category,rule.source,rule.configurable,rule.description));
        for reference in rule.spec_refs {
            rules.push_str(&format!(
                "CMSIS-SVD: [{}]({}).\n\n",
                reference.title, reference.url
            ));
        }
        if rule.svdconv_codes.is_empty() {
            rules.push_str("SVDConv relation: none claimed.\n\n");
        }
        for r in rule.svdconv_codes {
            rules.push_str(&format!("SVDConv: {} ({:?}).\n\n", r.code, r.compatibility));
        }
        for kind in ["negative", "positive", "boundary"] {
            if let Some(c) = manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["rule"] == rule.code && c["kind"] == kind)
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
        if !rule.code.starts_with("SEM") {
            if !manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["rule"] == rule.code && c["kind"] == "positive")
            {
                rules.push_str("Good SVD input: [minimal fixture](../tests/corpus/valid/minimal.svd). For runtime rules, this requires a readable UTF-8 file, an adequate normalization budget and, when enabled, a matching schema.\n\n");
            }
            if !manifest["cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["rule"] == rule.code && c["kind"] == "negative")
            {
                let example = match rule.code {
                    "IO001" => "svd-lint check missing.svd",
                    "IO002" => "svd-lint check non-utf8.svd",
                    "SVD007" => "svd-lint check short-description.svd --profile strict",
                    "XSD001" => "svd-lint check missing-schema-version.svd --xsd",
                    "XSD002" => "svd-lint check schema-invalid.svd --xsd --schema-dir schemas",
                    "XSD003" => "svd-lint check device.svd --xsd --schema-dir missing-schemas",
                    "XSD004" => "svd-lint check device.svd --xsd --schema-dir malformed-schemas",
                    _ => "svd-lint check invalid.svd",
                };
                rules.push_str(&format!(
                    "Bad runtime example:\n\n```console\n{example}\n```\n\n"
                ));
            }
            rules.push_str("Test coverage: diagnostic code round-trip, registry completeness, plus frontend/normalization, CLI I/O or schema contract tests as applicable. Runtime failures and XSD backend errors are not fabricated as semantic SVD failures.\n\n");
        }
    }
    let mut coverage = String::from(
        "# SVDConv coverage\n\nGenerated by `cargo xtask generate-docs`; do not edit.\n\nCatalogue scope: [official CMSIS-SVD 1.3.9 SVDConv message list](https://arm-software.github.io/CMSIS_5/SVD/html/svd_SVDConv_pg.html). Newer unlisted tool codes are not implicitly covered. Compatibility concerns checks, not identical severity or every tool mode.\n\n| SVDConv | svd-lint | Status | Compatibility | Notes |\n|---|---|---|---|---|\n",
    );
    let codes: Vec<_> = [
        20, 21, 22, 23, 24, 40, 41, 50, 51, 61, 101, 102, 103, 104, 105, 106, 107, 108, 109, 111,
        120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 132,
    ]
    .into_iter()
    .chain(201..=243)
    .chain(301..=378)
    .collect();
    let headers = [209, 219, 220, 305, 342, 371];
    for n in codes {
        let code = format!("M{n:03}");
        let mapped: Vec<_> = rule_registry()
            .iter()
            .flat_map(|r| {
                r.svdconv_codes
                    .iter()
                    .filter(|m| m.code == code)
                    .map(move |m| (r, m))
            })
            .collect();
        if mapped.is_empty() {
            let status = if n < 100 {
                "not-applicable"
            } else if n < 200 || [221, 222, 223, 226, 227].contains(&n) {
                "svdconv-specific"
            } else if headers.contains(&n) {
                "header-generation-only"
            } else {
                "planned"
            };
            coverage.push_str(&format!(
                "| {code} | — | {status} | — | No matching implemented check claimed |\n"
            ));
        } else {
            for (r, m) in mapped {
                coverage.push_str(&format!(
                    "| {code} | {} | implemented | {:?} | {} |\n",
                    r.code, m.compatibility, r.name
                ));
            }
        }
    }
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
