//! Описания диагностик и их классификация.
use crate::Severity;
use serde::Serialize;

/// Типы проверок
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleCategory {
    Structure,
    /// структура XML и обязательные части SVD файликов
    Normalization,
    /// наследование, массивы и ссылки
    Fields,
    /// ширина, границы и пересечения полей
    Registers,
    /// размеры и свойства регистров
    Addressing,
    /// адреса, перечисления регистров и адресные блоки
    Reset,
    /// значения после сброса и маски сброса
    Enumerations,
    /// ошибки в перечислениях (дубли, значения по умолчанию)
    Access,
    /// ошибки доступа (запись, чтение)
    Quality, // рекомендации по описанию
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum RuleSource {
    CmsisSvd,
    SvdConvCompatibility,
    SvdLint,
}

/// Метаданные проверки
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DiagnosticDefinition {
    pub code: crate::DiagnosticCode,
    /// наш внутренний код проверки
    pub name: &'static str,
    /// название её
    pub description: &'static str,
    /// и её описание
    pub category: RuleCategory,
    /// категория
    pub default_severity: Severity,
    /// тяжесть греха
    pub source: RuleSource,
    /// откуда проверка? SVDConv или наша
    pub configurable: bool,
    /// можно ли настраивать
    pub tags: &'static [&'static str],
}

impl DiagnosticDefinition {
    pub fn diagnostic(&self, message: impl Into<String>) -> crate::Diagnostic {
        crate::Diagnostic::new(self.code, self.default_severity, message)
    }

    /// The diagnostic identifier owned by this metadata.
    pub const fn code(&self) -> crate::DiagnosticCode {
        self.code
    }
}
