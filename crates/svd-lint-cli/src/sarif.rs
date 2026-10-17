//! SARIF presentation adapter. Core has no SARIF dependency.
use serde_json::{Value, json};
use svd_lint_core::{Diagnostic, SourceFile, Span, registry::rule_registry};
pub fn uri(path: &str) -> String {
    let p = path.replace('\\', "/");
    let mut encoded = String::new();
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b':' | b'-' | b'_' | b'.' | b'~') {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    if encoded.starts_with("//") {
        format!("file:{encoded}")
    } else if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else if encoded.as_bytes().get(1) == Some(&b':') {
        format!("file:///{encoded}")
    } else {
        encoded
    }
}
fn location(path: &str, span: Span, source: Option<&SourceFile>) -> Option<Value> {
    let source = source?;
    source.validate_span(span).ok()?;
    let start = source.line_col(span.start)?;
    let end = source.line_col(span.end)?;
    Some(
        json!({"physicalLocation":{"artifactLocation":{"uri":uri(path)},"region":{
            "startLine":start.line,"startColumn":start.column,"endLine":end.line,"endColumn":end.column
        }}}),
    )
}
pub fn results(path: &str, diagnostics: &[Diagnostic], source: Option<&SourceFile>) -> Vec<Value> {
    diagnostics
        .iter()
        .map(|d| {
            let mut result =
                json!({"ruleId":d.code.as_str(),"level":d.severity,"message":{"text":d.message}});
            if let Some(loc) = d.primary_span.and_then(|s| location(path, s, source)) {
                result["locations"] = json!([loc]);
            }
            let related: Vec<_> = d
                .related
                .iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    let mut loc = location(path, r.span, source)?;
                    loc["id"] = json!(i + 1);
                    loc["message"] =
                        json!({"text":r.message.as_deref().unwrap_or("related declaration")});
                    Some(loc)
                })
                .collect();
            if !related.is_empty() {
                result["relatedLocations"] = json!(related);
            }
            result
        })
        .collect()
}
pub fn document(results: Vec<Value>) -> Value {
    let rules:Vec<_>=rule_registry().iter().map(|r|json!({
        "id":r.code,"name":r.name,"shortDescription":{"text":r.description},
        "helpUri":r.spec_refs.first().map(|s|s.url).unwrap_or("https://github.com/Newlifer/svd-lint/blob/main/docs/rules.md"),
        "defaultConfiguration":{"level":r.default_severity},
        "properties":{"category":r.category,"source":r.source,"svdconv":r.svdconv_codes}
    })).collect();
    json!({"$schema":"https://json.schemastore.org/sarif-2.1.0.json","version":"2.1.0","runs":[{
        "tool":{"driver":{"name":"svd-lint","version":env!("CARGO_PKG_VERSION"),"rules":rules}},
        "columnKind":"unicodeCodePoints","results":results
    }]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_encode_windows_unicode_and_unc() {
        assert_eq!(uri("C:\\dir\\a b.svd"), "file:///C:/dir/a%20b.svd");
        assert_eq!(uri("\\\\server\\share\\a.svd"), "file://server/share/a.svd");
        assert_eq!(uri("a#%.svd"), "a%23%25.svd");
        assert_eq!(uri("т.svd"), "%D1%82.svd");
    }
}
