use crate::SourceFile;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaVersion(pub String);
impl SchemaVersion {
    pub fn detect(source: &SourceFile) -> Option<Self> {
        let doc = roxmltree::Document::parse(source.text()).ok()?;
        doc.root_element()
            .attribute("schemaVersion")
            .map(|v| Self(v.to_owned()))
    }
    pub fn supported(&self) -> bool {
        matches!(self.0.as_str(), "1.0" | "1.1" | "1.2" | "1.3")
            || self
                .0
                .strip_prefix("1.3.")
                .is_some_and(|v| v.parse::<u32>().is_ok_and(|n| (1..=12).contains(&n)))
    }
}
pub struct DirectoryProvider {
    pub directory: PathBuf,
}
impl DirectoryProvider {
    pub fn schema(&self, version: &SchemaVersion) -> Option<PathBuf> {
        if !version.supported() {
            return None;
        }
        // The family marker 1.3 selects the latest revision in the pinned set.
        // An explicitly supplied patch version always resolves exactly.
        let revision = if version.0 == "1.3" {
            "1.3.12"
        } else {
            &version.0
        };
        [
            format!("CMSIS-SVD_{}.xsd", revision),
            format!("CMSIS-SVD_{}.xsd", revision.replace('.', "_")),
        ]
        .into_iter()
        .map(|n| self.directory.join(n))
        .find(|p| p.is_file())
    }
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            directory: directory.as_ref().to_owned(),
        }
    }
}
