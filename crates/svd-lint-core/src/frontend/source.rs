//! Source file storage and byte-offset ↔ line/column conversion.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};

use crate::diagnostics::{Span, SpanError};

static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(0);

/// Identifier of a [`SourceFile`].
///
/// Ids are unique within a process. In later stages they will tie Canonical
/// IR nodes back to the XML nodes of a specific source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceId(pub u32);

impl SourceId {
    /// Allocates a fresh, process-unique source id.
    pub fn fresh() -> Self {
        SourceId(NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// 1-based line/column position in a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in Unicode scalar values (chars).
    pub column: u32,
}

/// A source file: its name and the full UTF-8 text.
///
/// Line starts are precomputed, so [`SourceFile::line_col`] is a binary
/// search and never rescans the file from the beginning.
#[derive(Clone, Debug)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    /// Byte offsets of the start of each line; `line_starts[0] == 0`.
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Creates a source file and precomputes its line index.
    ///
    /// Lines are split on `\n`; a trailing `\r` (CRLF) stays part of the
    /// previous line's content and does not affect offset math.
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (idx, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            id: SourceId::fresh(),
            name: name.into(),
            text,
            line_starts,
        }
    }

    /// Process-unique id of this source.
    pub fn id(&self) -> SourceId {
        self.id
    }

    /// File name as given at construction (usually a path).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Full UTF-8 source text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Length of the source in bytes.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Returns `true` if the source is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Number of lines (an empty file has exactly one empty line).
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a byte offset to a 1-based line/column position.
    ///
    /// The column is counted in Unicode scalar values, not bytes, so
    /// non-ASCII text (e.g. Cyrillic) reports natural column numbers.
    ///
    /// Returns `None` when `byte_offset` is past the end of the file or does
    /// not lie on a UTF-8 character boundary.
    pub fn line_col(&self, byte_offset: usize) -> Option<LineCol> {
        if byte_offset > self.text.len() || !self.text.is_char_boundary(byte_offset) {
            return None;
        }
        // Index of the last line start <= byte_offset.
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= byte_offset)
            - 1;
        let line_start = self.line_starts[line_idx];
        let column = self.text[line_start..byte_offset].chars().count() + 1;
        Some(LineCol {
            line: u32::try_from(line_idx + 1).ok()?,
            column: u32::try_from(column).ok()?,
        })
    }

    /// Converts a 1-based line/column position back to a byte offset.
    ///
    /// `column` is counted in Unicode scalar values, matching [`LineCol`].
    /// Returns `None` when the line does not exist or the column is past the
    /// end of that line.
    pub fn offset_at(&self, line: u32, column: u32) -> Option<usize> {
        if line == 0 || column == 0 {
            return None;
        }
        let line_start = *self
            .line_starts
            .get(usize::try_from(line).ok()?.checked_sub(1)?)?;
        let mut offset = line_start;
        for _ in 1..column {
            let ch = self.text.get(offset..)?.chars().next()?;
            if ch == '\n' {
                return None;
            }
            offset += ch.len_utf8();
        }
        Some(offset)
    }

    /// Validates a span against this source: bounds and UTF-8 boundaries.
    pub fn validate_span(&self, span: Span) -> Result<(), SpanError> {
        if span.start > span.end {
            return Err(SpanError::StartAfterEnd {
                start: span.start,
                end: span.end,
            });
        }
        if span.end > self.text.len() {
            return Err(SpanError::OutOfBounds {
                end: span.end,
                len: self.text.len(),
            });
        }
        for offset in [span.start, span.end] {
            if !self.text.is_char_boundary(offset) {
                return Err(SpanError::NotCharBoundary { offset });
            }
        }
        Ok(())
    }

    /// Extracts the source text covered by a span, if it is valid.
    pub fn snippet(&self, span: Span) -> Option<&str> {
        self.validate_span(span).ok()?;
        self.text.get(span.to_range())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_has_single_line() {
        let src = SourceFile::new("empty.svd", "");
        assert_eq!(src.line_count(), 1);
        assert_eq!(src.line_col(0), Some(LineCol { line: 1, column: 1 }));
        assert_eq!(src.line_col(1), None);
    }

    #[test]
    fn single_line_positions() {
        let src = SourceFile::new("a.svd", "abc");
        assert_eq!(src.line_col(0), Some(LineCol { line: 1, column: 1 }));
        assert_eq!(src.line_col(2), Some(LineCol { line: 1, column: 3 }));
        assert_eq!(src.line_col(3), Some(LineCol { line: 1, column: 4 }));
        assert_eq!(src.line_col(4), None);
    }

    #[test]
    fn multiline_positions() {
        let src = SourceFile::new("a.svd", "one\ntwo\nthree");
        assert_eq!(src.line_count(), 3);
        assert_eq!(src.line_col(0), Some(LineCol { line: 1, column: 1 }));
        // 't' of "two"
        assert_eq!(src.line_col(4), Some(LineCol { line: 2, column: 1 }));
        assert_eq!(src.line_col(6), Some(LineCol { line: 2, column: 3 }));
        // 't' of "three"
        assert_eq!(src.line_col(8), Some(LineCol { line: 3, column: 1 }));
        // newline itself belongs to its line
        assert_eq!(src.line_col(3), Some(LineCol { line: 1, column: 4 }));
    }

    #[test]
    fn crlf_line_endings() {
        // offsets: a=0 b=1 \r=2 \n=3 | c=4 d=5 \r=6 \n=7
        let src = SourceFile::new("a.svd", "ab\r\ncd\r\n");
        assert_eq!(src.line_count(), 3);
        assert_eq!(src.line_col(0), Some(LineCol { line: 1, column: 1 }));
        assert_eq!(src.line_col(2), Some(LineCol { line: 1, column: 3 })); // '\r'
        assert_eq!(src.line_col(4), Some(LineCol { line: 2, column: 1 })); // 'c'
        assert_eq!(src.line_col(5), Some(LineCol { line: 2, column: 2 })); // 'd'
        assert_eq!(src.line_col(8), Some(LineCol { line: 3, column: 1 }));
        assert_eq!(src.offset_at(2, 1), Some(4));
    }

    #[test]
    fn utf8_columns_count_chars() {
        // 'б' is 2 bytes; 'я' too.
        let src = SourceFile::new("a.svd", "абв\nгде");
        assert_eq!(src.line_col(0), Some(LineCol { line: 1, column: 1 }));
        assert_eq!(src.line_col(2), Some(LineCol { line: 1, column: 2 })); // 'б'
        assert_eq!(src.line_col(4), Some(LineCol { line: 1, column: 3 })); // 'в'
        assert_eq!(src.line_col(7), Some(LineCol { line: 2, column: 1 })); // 'г'
        // offset 1 is inside 'а' — not a char boundary.
        assert_eq!(src.line_col(1), None);
        assert_eq!(src.offset_at(1, 3), Some(4));
        assert_eq!(src.offset_at(2, 2), Some(9));
    }

    #[test]
    fn offset_at_rejects_bad_positions() {
        let src = SourceFile::new("a.svd", "ab\ncd");
        assert_eq!(src.offset_at(0, 1), None);
        assert_eq!(src.offset_at(1, 0), None);
        assert_eq!(src.offset_at(3, 1), None);
        // column past end of line 1 (line 1 is "ab", 2 chars + newline)
        assert_eq!(src.offset_at(1, 4), None);
        assert_eq!(src.offset_at(2, 3), Some(5)); // end of "cd"
    }

    #[test]
    fn validate_span_checks_bounds_and_boundaries() {
        let src = SourceFile::new("a.svd", "аб");
        assert!(src.validate_span(Span::new(0, 4).unwrap()).is_ok());
        assert!(src.validate_span(Span::new(0, 2).unwrap()).is_ok());
        assert_eq!(
            src.validate_span(Span::new(0, 5).unwrap()),
            Err(SpanError::OutOfBounds { end: 5, len: 4 })
        );
        assert_eq!(
            src.validate_span(Span::new(1, 3).unwrap()),
            Err(SpanError::NotCharBoundary { offset: 1 })
        );
        assert_eq!(
            src.validate_span(Span { start: 3, end: 2 }),
            Err(SpanError::StartAfterEnd { start: 3, end: 2 })
        );
    }

    #[test]
    fn snippet_extracts_text() {
        let src = SourceFile::new("a.svd", "<name>Тест</name>");
        let start = src.text().find('Т').unwrap();
        let span = Span::new(start, start + "Тест".len()).unwrap();
        assert_eq!(src.snippet(span), Some("Тест"));
        assert_eq!(src.snippet(Span::new(0, 10_000).unwrap()), None);
    }

    #[test]
    fn source_ids_are_unique() {
        let a = SourceFile::new("a", "");
        let b = SourceFile::new("b", "");
        assert_ne!(a.id(), b.id());
    }
}
