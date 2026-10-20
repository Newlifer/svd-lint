use clap::ValueEnum;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
use svd_lint_core::{Diagnostic, Severity, registry::find_rule};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    #[default]
    Default,
    Strict,
    Svdconv,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Allow,
    Warning,
    Error,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleOptions {
    pub enabled: Option<bool>,
    pub severity: Option<Level>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub files: Vec<String>,
    pub rules: Vec<String>,
    pub severity: Level,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub profile: Profile,
    #[serde(default)]
    pub rules: BTreeMap<String, RuleOptions>,
    #[serde(default)]
    pub overrides: Vec<Override>,
}
fn validate_rule(code: &str, options: &RuleOptions) -> Result<(), String> {
    let rule = find_rule(code).ok_or_else(|| format!("unknown rule {code}"))?;
    if !rule.configurable
        && (options.enabled == Some(false) || matches!(options.severity, Some(Level::Allow)))
    {
        return Err(format!(
            "{code} is required for analysis and cannot be disabled"
        ));
    }
    let requested = match options.severity {
        Some(Level::Error) => Some(Severity::Error),
        Some(Level::Warning) => Some(Severity::Warning),
        _ => None,
    };
    if requested.is_some_and(|severity| severity != rule.severity) {
        return Err(format!(
            "{code} has fixed severity {:?}; only suppression is configurable",
            rule.severity
        ));
    }
    Ok(())
}
impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        let default = Path::new("svd-lint.toml");
        let path = path.or_else(|| default.is_file().then_some(default));
        let config: Self = match path {
            Some(p) => toml::from_str(
                &std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?,
            )
            .map_err(|e| e.to_string())?,
            None => Self::default(),
        };
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<(), String> {
        for (code, options) in &self.rules {
            validate_rule(code, options)?;
        }
        for item in &self.overrides {
            for file in &item.files {
                globset::Glob::new(file).map_err(|e| e.to_string())?;
            }
            for code in &item.rules {
                validate_rule(
                    code,
                    &RuleOptions {
                        enabled: None,
                        severity: Some(item.severity),
                    },
                )?;
            }
        }
        Ok(())
    }
    pub fn apply(&self, diagnostics: &mut Vec<Diagnostic>, path: &str) {
        diagnostics.retain(|d| {
            let code = d.code().as_str();
            let configurable = find_rule(code).is_some_and(|r| r.configurable);
            if !configurable {
                return true;
            }
            let mut enabled = true;
            if let Some(options) = self.rules.get(code) {
                if let Some(value) = options.severity {
                    enabled = !matches!(value, Level::Allow);
                }
                if options.enabled == Some(false) {
                    enabled = false;
                }
            }
            let normalized = path.replace('\\', "/");
            for item in &self.overrides {
                if item.rules.iter().any(|c| c == code)
                    && item.files.iter().any(|g| {
                        globset::Glob::new(g)
                            .expect("validated glob")
                            .compile_matcher()
                            .is_match(&normalized)
                    })
                {
                    enabled = !matches!(item.severity, Level::Allow);
                }
            }
            enabled
        });
    }
}
