#![no_main]
use libfuzzer_sys::fuzz_target;
use svd_lint_core::*;
fuzz_target!(|data: &[u8]| {
    if data.len() > 65_536 {
        return;
    }
    if let Ok(text) = std::str::from_utf8(data) {
        let source = SourceFile::new("fuzz.svd", text);
        let _ = parse_svd(&source);
    }
});
