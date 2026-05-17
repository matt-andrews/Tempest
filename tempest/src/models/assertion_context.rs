use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct AssertionContext {
    /// Root directory passed to `tempest test`.
    pub suite_dir: PathBuf,

    /// Path to the current `.spec.yml`, if known.
    pub spec_file: Option<PathBuf>,
}

impl AssertionContext {
    pub fn resolve_file(&self, raw: &str) -> Result<PathBuf> {
        let raw = raw.trim();

        if raw.is_empty() {
            bail!("fileBytes path cannot be empty");
        }

        let relative = raw.trim_start_matches("file:");

        let base = if relative.starts_with('/') {
            // Tempest-root relative:
            // fileBytes("/img.jpg") -> <suite_dir>/img.jpg
            &self.suite_dir
        } else {
            // Spec-file relative:
            // fileBytes("img.jpg") -> <spec-dir>/img.jpg
            self.spec_file
                .as_deref()
                .and_then(Path::parent)
                .unwrap_or(&self.suite_dir)
        };

        let relative = relative.trim_start_matches('/');
        let resolved = normalize_under(base, Path::new(relative))
            .with_context(|| format!("invalid fileBytes path: {raw}"))?;

        if !resolved.is_file() {
            bail!("fileBytes could not find file: {}", resolved.display());
        }

        Ok(resolved)
    }
}

fn normalize_under(base: &Path, relative: &Path) -> Result<PathBuf> {
    let mut out = base.to_path_buf();

    for component in relative.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() || !out.starts_with(base) {
                    bail!("path escapes fixture root");
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!("absolute filesystem paths are not allowed");
            }
        }
    }

    Ok(out)
}
