use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// Root directory passed to `tempest test`.
    pub suite_dir: PathBuf,

    /// Path to the current `.spec.yml`, if known.
    pub spec_file: Option<PathBuf>,
}

impl EvaluationContext {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_file(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, b"x").unwrap();
        path
    }

    fn suite(dir: &TempDir) -> EvaluationContext {
        EvaluationContext {
            suite_dir: dir.path().to_path_buf(),
            spec_file: None,
        }
    }

    #[test]
    fn empty_path_returns_error() {
        let err = EvaluationContext::default().resolve_file("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn whitespace_only_path_returns_error() {
        assert!(EvaluationContext::default().resolve_file("   ").is_err());
    }

    #[test]
    fn root_relative_resolves_under_suite_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file = make_file(&dir, "asset.bin");
        assert_eq!(suite(&dir).resolve_file("/asset.bin").unwrap(), file);
    }

    #[test]
    fn spec_relative_resolves_under_spec_parent() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("specs");
        std::fs::create_dir(&sub).unwrap();
        let file = sub.join("data.bin");
        std::fs::write(&file, b"y").unwrap();

        let ctx = EvaluationContext {
            suite_dir: dir.path().to_path_buf(),
            spec_file: Some(sub.join("test.spec.yml")),
        };
        assert_eq!(ctx.resolve_file("data.bin").unwrap(), file);
    }

    #[test]
    fn spec_relative_falls_back_to_suite_dir_when_no_spec_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = make_file(&dir, "fallback.bin");
        assert_eq!(suite(&dir).resolve_file("fallback.bin").unwrap(), file);
    }

    #[test]
    fn file_colon_prefix_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let file = make_file(&dir, "data.bin");
        assert_eq!(suite(&dir).resolve_file("file:data.bin").unwrap(), file);
    }

    #[test]
    fn root_relative_file_colon_prefix_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let file = make_file(&dir, "data.bin");
        assert_eq!(suite(&dir).resolve_file("file:/data.bin").unwrap(), file);
    }

    #[test]
    fn traversal_escaping_root_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = suite(&dir).resolve_file("/../etc/passwd").unwrap_err();
        assert!(
            err.to_string().contains("escapes") || err.to_string().contains("invalid"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn traversal_within_root_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let file = make_file(&dir, "sibling.bin");

        // sub/../sibling.bin stays within suite_dir and should resolve correctly
        assert_eq!(
            suite(&dir).resolve_file("sub/../sibling.bin").unwrap(),
            file
        );
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(suite(&dir).resolve_file("/not_there.bin").is_err());
    }
}
