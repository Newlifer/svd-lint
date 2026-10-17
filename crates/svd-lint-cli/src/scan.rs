use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub fn collect(
    paths: &[PathBuf],
    recursive: bool,
    excludes: &[String],
) -> Result<Vec<PathBuf>, String> {
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in excludes {
        builder.add(globset::Glob::new(pattern).map_err(|e| e.to_string())?);
    }
    let excludes = builder.build().map_err(|e| e.to_string())?;
    let mut files = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for path in paths {
        walk(path, recursive, true, &excludes, &mut files, &mut visited)?;
    }
    Ok(files.into_iter().collect())
}
fn walk(
    path: &Path,
    recursive: bool,
    explicit: bool,
    excludes: &globset::GlobSet,
    files: &mut BTreeSet<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if excludes.is_match(&normalized) {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(path);
    let Ok(meta) = metadata else {
        if explicit {
            files.insert(path.to_owned());
        }
        return Ok(());
    };
    if meta.is_dir() {
        if !visited.insert(std::fs::canonicalize(path).map_err(|e| e.to_string())?) {
            return Ok(());
        }
        let mut children = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        children.sort();
        for child in children {
            if child.is_dir() && !recursive {
                continue;
            }
            if child.is_symlink() {
                continue;
            }
            walk(&child, recursive, false, excludes, files, visited)?;
        }
    } else if explicit
        || path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svd"))
    {
        let mut clean = PathBuf::new();
        for c in path.components() {
            if c != std::path::Component::CurDir {
                clean.push(c);
            }
        }
        files.insert(clean);
    }
    Ok(())
}
