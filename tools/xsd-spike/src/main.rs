//! Isolated compatibility gate. Never linked into svd-lint-core.
use serde_json::{Value, json};
use std::{fs, panic::catch_unwind, path::PathBuf};
fn fixture(version: &str) -> String {
    let cpu = if version == "1.0" {
        ""
    } else {
        "<cpu><name>CM3</name><revision>r0p0</revision><endian>little</endian><mpuPresent>false</mpuPresent><fpuPresent>false</fpuPresent><nvicPrioBits>4</nvicPrioBits><vendorSystickConfig>false</vendorSystickConfig></cpu>"
    };
    format!(
        "<device schemaVersion=\"{version}\"><name>D</name><version>1</version><description>Device</description>{cpu}<addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><peripherals><peripheral><name>P</name><description>Peripheral</description><baseAddress>0</baseAddress><addressBlock><offset>0</offset><size>4</size><usage>registers</usage></addressBlock><registers><register><name>R</name><description>Register</description><addressOffset>0</addressOffset><size>32</size></register></registers></peripheral></peripherals></device>"
    )
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--case") {
        let path = PathBuf::from(&args[1]);
        let backend = args[2].as_str();
        let schema = fs::read_to_string(&path).unwrap();
        let filename = path.file_name().unwrap().to_string_lossy();
        let version = if filename.contains("_1_0.") {
            "1.0"
        } else if filename.contains("_1_1.") {
            "1.1"
        } else if filename.contains("_1_2.") {
            "1.2"
        } else {
            "1.3"
        };
        let valid = fixture(version);
        let mut inputs = vec![
            ("valid", valid.clone(), true),
            ("missing-name", valid.replace("<name>D</name>", ""), false),
            (
                "bad-number",
                valid.replace(
                    "<baseAddress>0</baseAddress>",
                    "<baseAddress>oops</baseAddress>",
                ),
                false,
            ),
            (
                "unknown-element",
                valid.replace("</register>", "<unknown/></register>"),
                false,
            ),
            (
                "missing-register-offset",
                valid.replace("<addressOffset>0</addressOffset>", ""),
                false,
            ),
        ];
        if version == "1.3" {
            let fields = "<fields><field><dim>2</dim><dimIncrement>2</dimIncrement><name>F%s</name><bitOffset>0</bitOffset><bitWidth>2</bitWidth><enumeratedValues><name>E</name><enumeratedValue><name>W</name><value>#1x</value></enumeratedValue><enumeratedValue><name>D</name><isDefault>true</isDefault></enumeratedValue></enumeratedValues></field></fields>";
            let modern = valid
                .replace(
                    "<register><name>R</name>",
                    "<register><dim>2</dim><dimIncrement>4</dimIncrement><name>R%s</name>",
                )
                .replace("</register>", &format!("{fields}</register>"))
                .replace("<size>4</size>", "<size>8</size>");
            let modern = if filename == "CMSIS-SVD_1_3_1.xsd" {
                modern.replace(
                    "<field><dim>2</dim><dimIncrement>2</dimIncrement><name>F%s</name>",
                    "<field><name>F</name>",
                )
            } else {
                modern
            };
            inputs.push(("valid-enums-and-arrays", modern, true));
        }

        let run = catch_unwind(|| -> Result<Vec<Value>, String> {
            if backend == "uppsala" {
                let doc = uppsala::parse(&schema).map_err(|e| e.to_string())?;
                let validator =
                    uppsala::XsdValidator::from_schema(&doc).map_err(|e| e.to_string())?;
                inputs.iter().map(|(name,xml,expected)|{
let doc=uppsala::parse(xml).map_err(|e|e.to_string())?;
let errors=validator.validate(&doc);
Ok(json!({"case":name,"accepted":errors.is_empty(),"expected_accepted":expected,"passed":errors.is_empty()==*expected,"diagnostics":format!("{errors:?}"),"position":null}))
}).collect()
            } else {
                let compiled = xmlschema::parse_schema(&schema).map_err(|e| e.to_string())?;
                inputs.iter().map(|(name,xml,expected)| {
let doc=oxml::parse(xml).map_err(|e|e.to_string())?;
let report=xmlschema::validate(&doc,&compiled);
Ok(json!({"case":name,"accepted":report.is_valid(),"expected_accepted":expected,"passed":report.is_valid()==*expected,"diagnostics":format!("{:?}",report.violations),"position":null}))
}).collect()
            }
        });
        let mut row = json!({"schema":filename,"backend":backend,"license":if schema.contains("SPDX-License-Identifier: Apache-2.0"){"Apache-2.0"}else{"requires per-file license review"}});
        match run {
            Ok(Ok(cases)) => {
                row["compiled"] = json!(true);
                row["passed"] = json!(cases.iter().all(|c| c["passed"] == true));
                row["cases"] = json!(cases);
            }
            Ok(Err(e)) => {
                row["compiled"] = json!(false);
                row["passed"] = json!(false);
                row["error"] = json!(e);
            }
            Err(_) => {
                row["compiled"] = json!(false);
                row["passed"] = json!(false);
                row["panic"] = json!(true);
            }
        }

        println!("{}", serde_json::to_string(&row).unwrap());
        return;
    }
    let dir = PathBuf::from(args.first().expect("schema directory"));
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "xsd"))
        .collect();
    paths.sort();
    let mut rows = Vec::new();
    for path in paths {
        for backend in ["uppsala", "xmlschema"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--case")
                .arg(&path)
                .arg(backend)
                .output()
                .unwrap();
            let row = if output.status.success() {
                serde_json::from_slice::<Value>(&output.stdout).unwrap()
            } else {
                let stack_overflow =
                    String::from_utf8_lossy(&output.stderr).contains("overflowed its stack");
                json!({"schema":path.file_name().unwrap().to_string_lossy(),"backend":backend,"compiled":false,"passed":false,"process_failure":if stack_overflow {"stack-overflow"} else {"aborted"},"error":if stack_overflow {"stack overflow"} else {"candidate process terminated without a result"}})
            };
            rows.push(row);
        }
    }
    let qualified: Vec<_> = ["uppsala", "xmlschema"]
        .into_iter()
        .filter(|b| {
            !rows.is_empty()
                && rows
                    .iter()
                    .filter(|r| r["backend"] == *b)
                    .all(|r| r["passed"] == true)
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"version":1,"qualified_backends":qualified,"results":rows})
        )
        .unwrap()
    );
    if !qualified.contains(&"uppsala") {
        std::process::exit(1);
    }
}
