//! Каталог описаний диагностик для CLI, конфигурации и форматов вывода.
use crate::diagnostic_definition::DiagnosticDefinition;

mod diagnostics;
use diagnostics::DIAGNOSTICS;

pub fn rule_registry() -> &'static [&'static DiagnosticDefinition] {
    DIAGNOSTICS
}

pub fn find_rule(code: &str) -> Option<&'static DiagnosticDefinition> {
    DIAGNOSTICS
        .iter()
        .copied()
        .find(|definition| definition.code.as_str() == code)
}
