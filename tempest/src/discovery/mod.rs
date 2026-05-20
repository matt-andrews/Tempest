use crate::discovery::parser::FileParser;
use crate::models::descriptor::Descriptor;
use crate::models::directory_node::DirectoryNode;
use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use include_dir::{Dir, include_dir};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

mod parser;

static BUILTIN_REPORTERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/builtin_reporters");

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub directory: DirectoryNode,
    pub templates: HashMap<String, ReportTemplate>,
}

fn discover_internal_templates() -> anyhow::Result<HashMap<String, ReportTemplate>> {
    let mut templates = HashMap::new();
    collect_embedded_templates(&BUILTIN_REPORTERS, &mut templates)?;
    Ok(templates)
}

fn collect_embedded_templates(
    dir: &Dir,
    templates: &mut HashMap<String, ReportTemplate>,
) -> anyhow::Result<()> {
    for file in dir.files() {
        let path = file.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !stem.ends_with(".template") {
            continue;
        }

        let Some(parser) = parser::parser_for(path) else {
            continue;
        };
        let template = parser.parse_embedded_report_template(file, dir)?;

        let key = stem.trim_end_matches(".template").to_lowercase();
        templates.insert(key, template);
    }

    for sub_dir in dir.dirs() {
        collect_embedded_templates(sub_dir, templates)?;
    }

    Ok(())
}

fn parse_env(path: &Path) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)?;
    let config: HashMap<String, String> = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            line.split_once('#')
                .map_or(line, |(before, _)| before)
                .trim()
        })
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            line.split_once('=')
                .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect();
    Ok(config)
}

pub fn discover(
    dir: &Path,
    inherited_configs: Option<Vec<RunOptions>>,
    inherited_envs: &mut HashMap<String, String>,
    run_path: &Path,
) -> anyhow::Result<DiscoveryResult> {
    let run_path_is_dir = run_path.is_dir();

    let (dirs, files): (Vec<_>, Vec<_>) = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .partition(|e| e.path().is_dir());

    let mut options: Vec<RunOptions> = inherited_configs.unwrap_or_default();
    let mut tests: Vec<Descriptor> = Vec::new();
    let mut templates: HashMap<String, ReportTemplate> = discover_internal_templates()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            HashMap::new()
        });

    for entry in files {
        let path = &entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if file_name.ends_with(".env") {
            inherited_envs.extend(parse_env(path)?);
            continue;
        }
        let Some(parser) = parser::parser_for(path) else {
            continue;
        };

        if stem.ends_with(".config") {
            options.push(parser.parse_config(path)?);
        } else if stem.ends_with(".spec") {
            match run_path_is_dir {
                true => {
                    if path.starts_with(run_path) {
                        tests.push(parser.parse_descriptor(path)?);
                    }
                }
                false => {
                    if path == run_path {
                        tests.push(parser.parse_descriptor(path)?);
                    }
                }
            };
        } else if stem.ends_with(".template") {
            let template = parser.parse_report_template(path)?;
            let key = stem.trim_end_matches(".template").to_lowercase();
            templates.insert(key, template);
        }
    }

    let mut children = Vec::new();
    for entry in dirs {
        let sub = discover(
            &entry.path(),
            Some(options.clone()),
            inherited_envs,
            run_path,
        )?;
        children.push(sub.directory);
        templates.extend(sub.templates);
    }

    Ok(DiscoveryResult {
        directory: DirectoryNode {
            files: tests,
            options,
            children,
            dir: dir.to_path_buf(),
            envs: inherited_envs.clone(),
        },
        templates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::run_options::RunOptions;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn discover_empty_directory_has_no_files_options_or_children() {
        let dir = tempdir().unwrap();
        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(result.directory.files.is_empty());
        assert!(result.directory.options.is_empty());
        assert!(result.directory.children.is_empty());
    }

    #[test]
    fn discover_result_dir_matches_input_path() {
        let dir = tempdir().unwrap();
        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert_eq!(result.directory.dir, dir.path());
    }

    #[test]
    fn discover_finds_spec_file_and_parses_descriptor() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("api.spec.yml"),
            "name: API Test\ntest:\n  route: /api/health\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert_eq!(result.directory.files.len(), 1);
        assert_eq!(result.directory.files[0].name.as_deref(), Some("API Test"));
    }

    #[test]
    fn discover_finds_multiple_spec_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.spec.yml"), "test:\n  route: /a\n").unwrap();
        fs::write(dir.path().join("b.spec.yml"), "test:\n  route: /b\n").unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert_eq!(result.directory.files.len(), 2);
    }

    #[test]
    fn discover_finds_config_file_and_parses_options() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("test.config.yml"),
            "base_uri: http://localhost:9090\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert_eq!(result.directory.options.len(), 1);
        assert_eq!(
            result.directory.options[0].base_uri.as_deref(),
            Some("http://localhost:9090")
        );
    }

    #[test]
    fn discover_finds_template_file_and_inserts_into_templates_map() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("custom.template.yml"),
            "test_template: inline content\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(result.templates.contains_key("custom"));
        assert_eq!(
            result.templates["custom"].test_template.as_deref(),
            Some("inline content")
        );
    }

    #[test]
    fn discover_template_key_is_lowercased_stem_without_template_suffix() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("MyReport.template.yml"),
            "test_template: x\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(
            result.templates.contains_key("myreport"),
            "key should be lowercased"
        );
    }

    #[test]
    fn discover_ignores_non_yaml_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("readme.txt"), "hello").unwrap();
        fs::write(dir.path().join("data.json"), "{}").unwrap();
        fs::write(dir.path().join("config.toml"), "[section]").unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(result.directory.files.is_empty());
        assert!(result.directory.options.is_empty());
    }

    #[test]
    fn discover_ignores_yaml_files_without_recognized_suffix() {
        let dir = tempdir().unwrap();
        // "data.yml" has stem "data" — no .spec/.config/.template suffix, so ignored
        fs::write(dir.path().join("data.yml"), "key: value\n").unwrap();
        fs::write(dir.path().join("setup.yaml"), "key: value\n").unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(result.directory.files.is_empty());
        assert!(result.directory.options.is_empty());
    }

    #[test]
    fn discover_recurses_into_subdirectory() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("test.spec.yml"),
            "name: SubTest\ntest:\n  route: /sub\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert_eq!(result.directory.children.len(), 1);
        assert_eq!(result.directory.children[0].files.len(), 1);
        assert_eq!(
            result.directory.children[0].files[0].name.as_deref(),
            Some("SubTest")
        );
    }

    #[test]
    fn discover_handles_run_path_dir() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.spec.yml"), "test:\n  route: /a\n").unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("test.spec.yml"),
            "name: SubTest\ntest:\n  route: /sub\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), &sub).unwrap();
        assert_eq!(result.directory.children.len(), 1);
        assert_eq!(result.directory.children[0].files.len(), 1);
        assert_eq!(
            result.directory.children[0].files[0].name.as_deref(),
            Some("SubTest")
        );
    }

    #[test]
    fn discover_handles_run_path_single() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("test.spec.yml"),
            "name: SubTest\ntest:\n  route: /sub\n",
        )
        .unwrap();
        fs::write(sub.join("a.spec.yml"), "test:\n  route: /a\n").unwrap();

        let result = discover(
            dir.path(),
            None,
            &mut HashMap::new(),
            &sub.join("test.spec.yml"),
        )
        .unwrap();
        assert_eq!(result.directory.children.len(), 1);
        assert_eq!(result.directory.children[0].files.len(), 1);
        assert_eq!(
            result.directory.children[0].files[0].name.as_deref(),
            Some("SubTest")
        );
    }

    #[test]
    fn discover_recurses_into_deeply_nested_directories() {
        let dir = tempdir().unwrap();
        let deep = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.spec.yml"), "test:\n  route: /deep\n").unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        // result -> child "a" -> child "b" -> child "c" with spec
        let a = &result.directory.children[0];
        let b = &a.children[0];
        let c = &b.children[0];
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.files[0].test.as_ref().unwrap().route, "/deep");
    }

    #[test]
    fn discover_inherits_parent_options_into_child_directory() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(
            dir.path().join("root.config.yml"),
            "base_uri: http://parent\n",
        )
        .unwrap();
        fs::write(sub.join("test.spec.yml"), "test:\n  route: /test\n").unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        let child = &result.directory.children[0];
        assert_eq!(child.options.len(), 1);
        assert_eq!(child.options[0].base_uri.as_deref(), Some("http://parent"));
    }

    #[test]
    fn discover_uses_provided_inherited_configs_as_starting_options() {
        let dir = tempdir().unwrap();
        let inherited = RunOptions {
            base_uri: Some("http://inherited".to_string()),
            debug: None,
            reports: None,
            start_time: None,
            retries: Some(0),
        };

        let result = discover(
            dir.path(),
            Some(vec![inherited]),
            &mut HashMap::new(),
            dir.path(),
        )
        .unwrap();
        assert_eq!(result.directory.options.len(), 1);
        assert_eq!(
            result.directory.options[0].base_uri.as_deref(),
            Some("http://inherited")
        );
    }

    #[test]
    fn discover_child_templates_are_merged_into_parent_result() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(
            sub.join("child.template.yml"),
            "test_template: from child\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(
            result.templates.contains_key("child"),
            "child templates should bubble up"
        );
    }

    #[test]
    fn discover_returns_error_for_nonexistent_directory() {
        let result = discover(
            &PathBuf::from("/no/such/directory/exists"),
            None,
            &mut HashMap::new(),
            &PathBuf::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn discover_always_includes_builtin_templates_even_in_empty_dir() {
        let dir = tempdir().unwrap();
        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(
            result.templates.contains_key("console"),
            "built-in templates should always be present"
        );
    }

    #[test]
    fn discover_user_template_does_not_overwrite_builtin_unless_same_key() {
        let dir = tempdir().unwrap();
        // A user template with key "myreport" does not affect "console"
        fs::write(
            dir.path().join("myreport.template.yml"),
            "test_template: custom\n",
        )
        .unwrap();

        let result = discover(dir.path(), None, &mut HashMap::new(), dir.path()).unwrap();
        assert!(result.templates.contains_key("console"));
        assert!(result.templates.contains_key("myreport"));
    }
}
