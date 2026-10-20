//! Описания диагностик.
use crate::Severity;
use serde::Serialize;

/// Метаданные проверки
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DiagnosticDefinition {
    pub code: crate::DiagnosticCode,
    /// наш внутренний код проверки
    pub name: &'static str,
    /// название её
    pub description: &'static str,
    /// Фиксированный уровень диагностики.
    pub severity: Severity,
    /// тяжесть греха
    pub configurable: bool,
    /// можно ли настраивать
    pub tags: &'static [&'static str],
}

impl DiagnosticDefinition {
    pub fn diagnostic(&self, message: impl Into<String>) -> crate::Diagnostic {
        crate::Diagnostic::new(self.code, message)
    }

    /// The diagnostic identifier owned by this metadata.
    pub const fn code(&self) -> crate::DiagnosticCode {
        self.code
    }
}
