//! Общий каталог всех проверок. Для списков в cli, вывода help и так далее.
use crate::Severity;
use serde::Serialize;

/// Типы проверок
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleCategory {
    Structure, /// структура XML и обязательные части SVD файликов
    Normalization, /// наследование, массивы и ссылки
    Fields, /// ширина, границы и пересечения полей
    Registers, /// размеры и свойства регистров
    Addressing, /// адреса, перечисления регистров и адресные блоки
    Reset, /// значения после сброса и маски сброса
    Enumerations, /// ошибки в перечислениях (дубли, значения по умолчанию)
    Access, /// ошибки доступа (запись, чтение)
    Quality, // рекомендации по описанию
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum RuleSource {
    CmsisSvd,
    SvdConvCompatibility,
    SvdLint,
}

/// Совместимость наших проверок с SVDConv
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Compatibility {
    Exact,
    Equivalent,
    Partial,
    Related,
}


/// Ссылка на спецификацию
#[derive(Clone, Debug, Serialize)]
pub struct SpecReference {
    pub title: &'static str,
    pub url: &'static str,
}

/// Ссылка на спецификацию SVDConv
#[derive(Clone, Debug, Serialize)]
pub struct SvdConvReference {
    pub code: &'static str,
    pub compatibility: Compatibility,
}

/// Метаданные проверки
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DiagnosticDefinition {
    pub code: crate::DiagnosticCode, /// наш внутренний код проверки
    pub name: &'static str, /// название её
    pub description: &'static str, /// и её описание
    pub category: RuleCategory, /// категория
    pub default_severity: Severity, /// тяжесть греха
    pub source: RuleSource, /// откуда проверка? SVDConv или наша
    pub configurable: bool, /// можно ли настраивать
    pub svdconv_codes: &'static [SvdConvReference], // совместимые коды SVDConv
    pub spec_refs: &'static [SpecReference],
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

enum Entry {
    Pipeline(&'static DiagnosticDefinition),
    Check(&'static crate::rules::Rule, bool),
}

impl Entry {
    const fn passive(metadata: &'static DiagnosticDefinition) -> Self {
        Self::Pipeline(metadata)
    }
    const fn active(rule: &'static crate::rules::Rule) -> Self {
        Self::Check(rule, true)
    }
    const fn optional(rule: &'static crate::rules::Rule) -> Self {
        Self::Check(rule, false)
    }
}

static ENTRIES: &[Entry] = &[
    Entry::optional(&crate::rules::quality::LOW_QUALITY_DESCRIPTION_CHECK),
    Entry::passive(&crate::frontend::diagnostics::IO_READ_FAILED),
    Entry::passive(&crate::frontend::diagnostics::IO_INVALID_UTF8),
    Entry::passive(&crate::frontend::diagnostics::XML_SYNTAX),
    Entry::passive(&crate::frontend::diagnostics::INVALID_ROOT_ELEMENT),
    Entry::passive(&crate::frontend::diagnostics::MISSING_REQUIRED_ELEMENT),
    Entry::passive(&crate::frontend::diagnostics::DUPLICATE_SINGLETON_ELEMENT),
    Entry::passive(&crate::frontend::diagnostics::EMPTY_REQUIRED_ELEMENT),
    Entry::passive(&crate::frontend::diagnostics::MISSING_PERIPHERAL_DESCRIPTION),
    Entry::passive(&crate::frontend::diagnostics::SVD_PARSE_ERROR),
    Entry::passive(&crate::normalize::diagnostics::MISSING_REFERENCE),
    Entry::passive(&crate::normalize::diagnostics::CYCLE),
    Entry::passive(&crate::normalize::diagnostics::AMBIGUOUS_REFERENCE),
    Entry::passive(&crate::normalize::diagnostics::INVALID_DIMENSION),
    Entry::passive(&crate::normalize::diagnostics::INVALID_INDEX),
    Entry::passive(&crate::normalize::diagnostics::INVALID_TEMPLATE),
    Entry::passive(&crate::normalize::diagnostics::ADDRESS_OVERFLOW),
    Entry::passive(&crate::normalize::diagnostics::MISSING_PROPERTY),
    Entry::passive(&crate::normalize::diagnostics::EXPANSION_LIMIT),
    Entry::active(&crate::rules::registers::ZERO_REGISTER_SIZE_CHECK),
    Entry::active(&crate::rules::fields::ZERO_FIELD_WIDTH_CHECK),
    Entry::active(&crate::rules::fields::FIELD_OUT_OF_BOUNDS_CHECK),
    Entry::active(&crate::rules::fields::FIELD_OVERLAP_CHECK),
    Entry::active(&crate::rules::reset::RESET_VALUE_OUT_OF_BOUNDS_CHECK),
    Entry::active(&crate::rules::reset::RESET_MASK_OUT_OF_BOUNDS_CHECK),
    Entry::active(&crate::rules::reset::RESET_OUTSIDE_MASK_CHECK),
    Entry::active(&crate::rules::memory::REGISTER_OVERLAP_CHECK),
    Entry::active(&crate::rules::alternatives::INVALID_ALTERNATE_REGISTER_CHECK),
    Entry::active(&crate::rules::memory::REGISTER_OUTSIDE_BLOCK_CHECK),
    Entry::active(&crate::rules::memory::INVALID_ADDRESS_BLOCK_CHECK),
    Entry::active(&crate::rules::enums::ENUM_VALUE_OUT_OF_BOUNDS_CHECK),
    Entry::active(&crate::rules::enums::DUPLICATE_ENUM_ENTRY_CHECK),
    Entry::active(&crate::rules::enums::MULTIPLE_ENUM_DEFAULTS_CHECK),
    Entry::active(&crate::rules::access::INVALID_WRITE_CONSTRAINT_CHECK),
    Entry::active(&crate::rules::access::ACCESS_CONFLICT_CHECK),
    Entry::passive(&crate::schema::diagnostics::UNKNOWN_VERSION),
    Entry::passive(&crate::schema::diagnostics::NONCONFORMANCE),
    Entry::passive(&crate::schema::diagnostics::MISSING_SCHEMA),
    Entry::passive(&crate::schema::diagnostics::BACKEND_ERROR),
];

pub fn rule_registry() -> &'static [&'static DiagnosticDefinition] {
    static CATALOGUE: std::sync::OnceLock<Vec<&'static DiagnosticDefinition>> =
        std::sync::OnceLock::new();
    CATALOGUE.get_or_init(|| collect_metadata(ENTRIES))
}

fn collect_metadata(entries: &[Entry]) -> Vec<&'static DiagnosticDefinition> {
    let mut catalogue = Vec::new();
    for entry in entries {
        match entry {
            Entry::Pipeline(metadata) => catalogue.push(*metadata),
            Entry::Check(rule, _) => {
                catalogue.extend_from_slice(rule.diagnostics);
            }
        }
    }
    catalogue
}

pub fn find_rule(code: &str) -> Option<&'static DiagnosticDefinition> {
    rule_registry()
        .iter()
        .copied()
        .find(|metadata| metadata.code.as_str() == code)
}

/// All registered executable checks, including opt-in quality checks.
pub fn executable_rules() -> impl Iterator<Item = &'static crate::rules::Rule> {
    ENTRIES.iter().filter_map(|entry| match entry {
        Entry::Check(rule, _) => Some(*rule),
        Entry::Pipeline(_) => None,
    })
}

pub(crate) fn default_rules() -> impl Iterator<Item = &'static crate::rules::Rule> {
    ENTRIES.iter().filter_map(|entry| match entry {
        Entry::Check(rule, true) => Some(*rule),
        _ => None,
    })
}
