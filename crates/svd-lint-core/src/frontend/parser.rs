//! Frontend entry point: XML preflight + typed `svd-parser` pass.

use super::diagnostics;
use svd_parser::svd::Device;

use crate::diagnostic::{Diagnostic, Span};

use super::preflight;
use super::source::SourceFile;
use super::source_map::SourceMap;

/// Result of the frontend stage.
///
/// `device` is `Some` only when the input passed XML parsing, preflight
/// checks and the typed `svd-parser` pass. Diagnostics are always in a
/// deterministic (source position) order.
#[derive(Debug)]
pub struct FrontendResult {
    /// Typed device model as produced by `svd-parser` (no derivedFrom
    /// expansion, no dim expansion, no normalization).
    pub device: Option<Device>,
    /// All diagnostics collected during this run.
    pub diagnostics: Vec<Diagnostic>,
    /// Source position index of the XML document, available whenever the
    /// XML was well-formed. Later stages use it to tie Canonical IR nodes
    /// back to XML source positions.
    pub source_map: Option<SourceMap>,
}

impl FrontendResult {
    /// Returns `true` when at least one diagnostic has [`Severity::Error`].
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity().is_error())
    }
}

/// Parses and checks one SVD source file.
///
/// Stages:
/// 1. XML syntax check via `roxmltree`. On failure a single `E003`
///    diagnostic is returned and the document is not processed further.
/// 2. Structural preflight checks (`E004`–`W002`). All problems are
///    collected.
/// 3. Typed parse via [`svd_parser::parse_with_config`] with validation
///    disabled and without `derivedFrom`/`dim` expansion. Only attempted
///    when preflight reported no errors, so messages are never duplicated.
///
/// This function never panics on malformed user input.
pub fn parse_svd(source: &SourceFile) -> FrontendResult {
    if let Some(span) = excessive_nesting(source.text()) {
        let mut diagnostic = Diagnostic::new(
            diagnostics::SVD_PARSE_ERROR.code(),
            "XML element nesting exceeds the analysis limit of 64",
        );
        diagnostic.primary_span = Some(span);
        return FrontendResult {
            device: None,
            diagnostics: vec![diagnostic],
            source_map: None,
        };
    }
    let document = match roxmltree::Document::parse(source.text()) {
        Ok(document) => document,
        Err(err) => {
            return FrontendResult {
                device: None,
                diagnostics: vec![xml_error_diagnostic(source, &err)],
                source_map: None,
            };
        }
    };

    let mut diagnostics = Vec::new();

    let source_map = match SourceMap::from_document(source, &document) {
        Ok(map) => Some(map),
        Err(err) => {
            // Should never happen with roxmltree-produced ranges; report
            // instead of panicking.
            diagnostics.push(Diagnostic::new(
                diagnostics::XML_SYNTAX.code(),
                err.to_string(),
            ));
            None
        }
    };

    diagnostics.extend(preflight::check_document(&document));

    let device = if diagnostics.iter().any(|d| d.severity().is_error()) {
        // Skipping the typed pass here also guarantees that svd-parser
        // errors cannot duplicate preflight findings.
        None
    } else {
        typed_parse(source, &mut diagnostics)
    };

    diagnostics.sort_by_key(|d| d.primary_span.map(|s| (s.start, s.end)));

    FrontendResult {
        device,
        diagnostics,
        source_map,
    }
}

/// A lexical resource guard, not an XML validator. Skip quoted attributes,
/// comments, CDATA and processing instructions before counting tag nesting.
/// The recursive roxmltree tokenizer must not see an unbounded nesting depth.
fn excessive_nesting(text: &str) -> Option<Span> {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    let mut depth = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        let rest = &text[start..];
        let terminator = if rest.starts_with("<!--") {
            Some("-->")
        } else if rest.starts_with("<![CDATA[") {
            Some("]]>")
        } else if rest.starts_with("<?") {
            Some("?>")
        } else {
            None
        };
        if let Some(terminator) = terminator {
            cursor = start + rest.find(terminator)? + terminator.len();
            continue;
        }
        cursor += 1;
        let mut quote = None;
        while cursor < bytes.len() {
            let b = bytes[cursor];
            if let Some(q) = quote {
                if b == q {
                    quote = None;
                }
            } else if matches!(b, b'\'' | b'"') {
                quote = Some(b);
            } else if b == b'>' {
                break;
            }
            cursor += 1;
        }
        if cursor == bytes.len() {
            return None;
        }
        let tag = &text[start + 1..cursor];
        if tag.starts_with('/') {
            depth = depth.saturating_sub(1);
        } else if !tag.starts_with('!') && !tag.trim_end().ends_with('/') {
            depth += 1;
            if depth > 64 {
                return Span::new(start, cursor + 1).ok();
            }
        }
        cursor += 1;
    }
    None
}

fn typed_parse(source: &SourceFile, diagnostics: &mut Vec<Diagnostic>) -> Option<Device> {
    let mut config = svd_parser::Config::default();
    config.validate_level = svd_parser::ValidateLevel::Disabled;
    // Enumerated values and write constraints are parsed, not skipped.
    config.ignore_enums = false;
    match svd_parser::parse_with_config(source.text(), &config) {
        Ok(device) => Some(device),
        Err(err) => {
            // svd-parser does not expose source positions for its errors;
            // per policy no coordinates are invented.
            diagnostics.push(Diagnostic::new(
                diagnostics::SVD_PARSE_ERROR.code(),
                format!("failed to parse SVD: {err}"),
            ));
            None
        }
    }
}

fn xml_error_diagnostic(source: &SourceFile, err: &roxmltree::Error) -> Diagnostic {
    let span = if matches!(
        err,
        roxmltree::Error::UnexpectedEndOfStream | roxmltree::Error::UnclosedRootNode
    ) {
        // roxmltree reports (1,1) for these truncation errors; the honest
        // location of a truncated document is its end.
        Span::new(source.len(), source.len()).ok()
    } else {
        let pos = err.pos();
        source
            .offset_at(pos.row, pos.col)
            .and_then(|offset| Span::new(offset, offset).ok())
    };
    let mut diagnostic = Diagnostic::new(
        diagnostics::XML_SYNTAX.code(),
        format!("XML syntax error: {err}"),
    );
    diagnostic.primary_span = span;
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(xml: &str) -> FrontendResult {
        let source = SourceFile::new("test.svd", xml);
        parse_svd(&source)
    }

    const MINIMAL: &str = "\
<device schemaVersion=\"1.1\">
  <name>D</name>
  <version>1.0</version>
  <description>desc</description>
  <addressUnitBits>8</addressUnitBits>
  <width>32</width>
  <peripherals>
    <peripheral>
      <name>P0</name>
      <description>p</description>
      <baseAddress>0x40000000</baseAddress>
    </peripheral>
  </peripherals>
</device>
";

    #[test]
    fn valid_svd_parses_into_device() {
        let result = parse(MINIMAL);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let device = result.device.unwrap();
        assert_eq!(device.name, "D");
        assert_eq!(device.peripherals.len(), 1);
        assert!(result.source_map.is_some());
    }

    #[test]
    fn broken_xml_yields_xml001_and_stops() {
        let result = parse("<device><name>D</name>");
        assert!(result.device.is_none());
        assert!(result.source_map.is_none());
        assert_eq!(result.diagnostics.len(), 1);
        let diag = &result.diagnostics[0];
        assert_eq!(diag.code(), diagnostics::XML_SYNTAX.code());
        assert!(diag.primary_span.is_some());
    }

    #[test]
    fn structural_errors_skip_typed_parse() {
        // Missing <width> plus a value svd-parser would also reject; only
        // the preflight diagnostic must be reported (no duplicates).
        let result = parse(MINIMAL.replace("  <width>32</width>\n", "").as_str());
        assert!(result.device.is_none());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code(),
            diagnostics::MISSING_REQUIRED_ELEMENT.code()
        );
    }

    #[test]
    fn invalid_number_is_svd006_without_panic() {
        let result = parse(
            MINIMAL
                .replace("<width>32</width>", "<width>abc</width>")
                .as_str(),
        );
        assert!(result.device.is_none());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code(),
            diagnostics::SVD_PARSE_ERROR.code()
        );
        // No position is known, so no position is reported.
        assert!(result.diagnostics[0].primary_span.is_none());
    }

    #[test]
    fn xml_error_position_is_real() {
        let xml = "<device>\n  <name>D</name>\n</device";
        let source = SourceFile::new("t.svd", xml);
        let result = parse_svd(&source);
        let span = result.diagnostics[0].primary_span.unwrap();
        let pos = source.line_col(span.start).unwrap();
        assert_eq!(pos.line, 3);
    }
}
