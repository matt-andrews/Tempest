use crate::discovery::parser::FileParser;
use crate::models::descriptor::Descriptor;
use crate::models::directory_node::DirectoryNode;
use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use anyhow::Context;
use include_dir::{Dir, include_dir};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::models::project::Project;

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

pub fn parse_env(
    path: &Path,
    inherited_env: &HashMap<String, String>,
    engine: &LiquidEngine,
) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)?;
    let mut effective = inherited_env.clone();
    let mut config = HashMap::new();

    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();

        if line.starts_with('#') {
            continue;
        }

        let line = line
            .split_once('#')
            .map_or(line, |(before, _)| before)
            .trim();

        if line.is_empty() {
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim().to_owned();

        let value = {
            // Rebuild the context after every assignment so this line can
            // access values resolved earlier in the same file.
            let liquid_context = liquid::object!({
                "env": &effective,
            });

            engine.render(raw_value.trim(), &liquid_context)
        }
        .with_context(|| {
            format!(
                "failed to render environment variable `{key}` in {} at line {}",
                path.display(),
                line_index + 1
            )
        })?;

        effective.insert(key.clone(), value.clone());
        config.insert(key, value);
    }

    Ok(config)
}

pub fn discover_project(
    dir: &Path,
    project: Option<PathBuf>,
) -> anyhow::Result<Project>{

    if let Some(project) = project
        && let Some(parser) = parser::parser_for(project.as_path()){
        return parser.parse_project(project.as_path());
    }

    let files: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().is_dir())
        .map(|e| e.path())
        .collect();

    for path in files{
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(parser) = parser::parser_for(path.as_path()) else {
            continue;
        };
        if stem.ends_with(".project") {
            return parser.parse_project(path.as_path());
        }
    }
    Err(anyhow::anyhow!("no project found"))
}

pub fn discover(
    dir: &Path,
    inherited_configs: Option<Vec<RunOptions>>,
    inherited_envs: &mut HashMap<String, String>,
    run_paths: &[PathBuf],
    liquid_engine: &LiquidEngine,
) -> anyhow::Result<DiscoveryResult> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|entry| entry.path());
    let (dirs, files): (Vec<_>, Vec<_>) = entries.into_iter().partition(|e| e.path().is_dir());

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
            let local_envs = parse_env(path, inherited_envs, liquid_engine)?;
            inherited_envs.extend(local_envs);
            continue;
        }
        let Some(parser) = parser::parser_for(path) else {
            continue;
        };

        if stem.ends_with(".config") {
            options.push(parser.parse_config(path)?);
        } else if stem.ends_with(".spec") {
            let selected = run_paths.iter().any(|run_path| path.starts_with(run_path));
            if selected {
                tests.push(parser.parse_descriptor(path)?);
            }
        } else if stem.ends_with(".template") {
            let template = parser.parse_report_template(path)?;
            let key = stem.trim_end_matches(".template").to_lowercase();
            templates.insert(key, template);
        }
    }

    let mut children = Vec::new();
    for entry in dirs {
        let mut child_envs = inherited_envs.clone();
        let sub = discover(
            &entry.path(),
            Some(options.clone()),
            &mut child_envs,
            run_paths,
            liquid_engine,
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

    fn discover_all(dir: &Path) -> anyhow::Result<DiscoveryResult> {
        discover_for_test(dir, None, &mut HashMap::new(), &[dir.to_path_buf()])
    }

    fn discover_for_test(
        dir: &Path,
        inherited_configs: Option<Vec<RunOptions>>,
        inherited_envs: &mut HashMap<String, String>,
        run_paths: &[PathBuf],
    ) -> anyhow::Result<DiscoveryResult> {
        discover(
            dir,
            inherited_configs,
            inherited_envs,
            run_paths,
            &LiquidEngine,
        )
    }

    fn discovered_file_names(result: &DiscoveryResult) -> Vec<String> {
        let mut names = result
            .directory
            .walk()
            .flat_map(|directory| &directory.files)
            .filter_map(|descriptor| descriptor.file.as_ref()?.file_name()?.to_str())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn parse_env_contents(contents: &str) -> HashMap<String, String> {
        parse_env_contents_with_inherited(contents, &HashMap::new()).unwrap()
    }

    fn parse_env_contents_with_inherited(
        contents: &str,
        inherited_env: &HashMap<String, String>,
    ) -> anyhow::Result<HashMap<String, String>> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.env");
        fs::write(&path, contents).unwrap();
        parse_env(&path, inherited_env, &LiquidEngine)
    }

    #[test]
    fn parse_env_parses_key_value_pairs() {
        let env = parse_env_contents("HOST=localhost\nPORT=8080\n");

        assert_eq!(
            env,
            HashMap::from([
                ("HOST".to_owned(), "localhost".to_owned()),
                ("PORT".to_owned(), "8080".to_owned()),
            ])
        );
    }

    #[test]
    fn parse_env_trims_keys_values_and_blank_lines() {
        let env = parse_env_contents("\n  HOST = localhost  \n\tPORT\t=\t8080\t\n\n");

        assert_eq!(env.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(env.get("PORT").map(String::as_str), Some("8080"));
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn parse_env_ignores_comments_and_removes_inline_comments() {
        let env = parse_env_contents(
            "# file comment\n  # indented comment\nHOST=localhost # local server\nPORT=8080#default\n",
        );

        assert_eq!(env.get("HOST").map(String::as_str), Some("localhost"));
        assert_eq!(env.get("PORT").map(String::as_str), Some("8080"));
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn parse_env_preserves_equals_signs_in_values() {
        let env = parse_env_contents(
            "DATABASE_URL=postgres://user:password@host/db?sslmode=require\nTOKEN=a=b=c\n",
        );

        assert_eq!(
            env.get("DATABASE_URL").map(String::as_str),
            Some("postgres://user:password@host/db?sslmode=require")
        );
        assert_eq!(env.get("TOKEN").map(String::as_str), Some("a=b=c"));
    }

    #[test]
    fn parse_env_ignores_lines_without_an_equals_sign_and_accepts_empty_values() {
        let env = parse_env_contents("MALFORMED\nEMPTY=\nVALID=value\n");

        assert!(!env.contains_key("MALFORMED"));
        assert_eq!(env.get("EMPTY").map(String::as_str), Some(""));
        assert_eq!(env.get("VALID").map(String::as_str), Some("value"));
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn parse_env_uses_the_last_value_for_duplicate_keys() {
        let env = parse_env_contents("MODE=development\nMODE=production\n");

        assert_eq!(env.get("MODE").map(String::as_str), Some("production"));
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn parse_env_supports_crlf_line_endings_and_unicode_values() {
        let env = parse_env_contents("GREETING=olá\r\nEMOJI=🌩️\r\n");

        assert_eq!(env.get("GREETING").map(String::as_str), Some("olá"));
        assert_eq!(env.get("EMOJI").map(String::as_str), Some("🌩️"));
    }

    #[test]
    fn parse_env_renders_values_from_the_inherited_environment() {
        let inherited = HashMap::from([
            ("HOST".to_owned(), "api.example.com".to_owned()),
            ("VERSION".to_owned(), "v1".to_owned()),
        ]);

        let env = parse_env_contents_with_inherited(
            "URL=https://{{ env.HOST }}/{{ env.VERSION }}\n",
            &inherited,
        )
        .unwrap();

        assert_eq!(
            env.get("URL").map(String::as_str),
            Some("https://api.example.com/v1")
        );
        assert!(!env.contains_key("HOST"));
        assert!(!env.contains_key("VERSION"));
    }

    #[test]
    fn parse_env_renders_later_assignments_against_earlier_assignments() {
        let inherited = HashMap::from([("MY_VAR".to_owned(), "Hello".to_owned())]);

        let env = parse_env_contents_with_inherited(
            "MY_VAR={{ env.MY_VAR }}, World!\nRESULT_VAR=Greetings! {{ env.MY_VAR }}\n",
            &inherited,
        )
        .unwrap();

        assert_eq!(env.get("MY_VAR").map(String::as_str), Some("Hello, World!"));
        assert_eq!(
            env.get("RESULT_VAR").map(String::as_str),
            Some("Greetings! Hello, World!")
        );
    }

    #[test]
    fn parse_env_allows_later_lines_to_use_new_values_from_the_same_file() {
        let env = parse_env_contents(
            "HOST=api.example.com\nBASE_URL=https://{{ env.HOST }}\nHEALTH_URL={{ env.BASE_URL }}/health\n",
        );

        assert_eq!(
            env.get("HEALTH_URL").map(String::as_str),
            Some("https://api.example.com/health")
        );
    }

    #[test]
    fn parse_env_duplicate_key_templates_use_the_previous_assignment() {
        let env = parse_env_contents(
            "MODE=development\nMODE={{ env.MODE }}-local\nLABEL=Mode: {{ env.MODE }}\n",
        );

        assert_eq!(
            env.get("MODE").map(String::as_str),
            Some("development-local")
        );
        assert_eq!(
            env.get("LABEL").map(String::as_str),
            Some("Mode: development-local")
        );
    }

    #[test]
    fn parse_env_supports_liquid_filters() {
        let inherited = HashMap::from([("NAME".to_owned(), "tempest".to_owned())]);

        let env =
            parse_env_contents_with_inherited("DISPLAY_NAME={{ env.NAME | upcase }}\n", &inherited)
                .unwrap();

        assert_eq!(env.get("DISPLAY_NAME").map(String::as_str), Some("TEMPEST"));
    }

    #[test]
    fn parse_env_template_errors_include_key_file_and_line() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("broken.env");
        fs::write(&path, "VALID=value\nBROKEN={{ env.VALID\n").unwrap();

        let error = parse_env(&path, &HashMap::new(), &LiquidEngine).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("BROKEN"));
        assert!(message.contains("broken.env"));
        assert!(message.contains("line 2"));
    }

    #[test]
    fn parse_env_returns_an_error_when_the_file_does_not_exist() {
        let dir = tempdir().unwrap();

        let result = parse_env(
            &dir.path().join("missing.env"),
            &HashMap::new(),
            &LiquidEngine,
        );

        assert!(result.is_err());
    }

    #[test]
    fn parse_env_returns_an_error_for_non_utf8_input() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.env");
        fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();

        let result = parse_env(&path, &HashMap::new(), &LiquidEngine);

        assert!(result.is_err());
    }

    #[test]
    fn discover_empty_directory_has_no_files_options_or_children() {
        let dir = tempdir().unwrap();
        let result = discover_all(dir.path()).unwrap();
        assert!(result.directory.files.is_empty());
        assert!(result.directory.options.is_empty());
        assert!(result.directory.children.is_empty());
    }

    #[test]
    fn discover_result_dir_matches_input_path() {
        let dir = tempdir().unwrap();
        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
        assert_eq!(result.directory.files.len(), 1);
        assert_eq!(result.directory.files[0].name.as_deref(), Some("API Test"));
    }

    #[test]
    fn discover_finds_multiple_spec_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("b.spec.yml"), "test:\n  route: /b\n").unwrap();
        fs::write(dir.path().join("a.spec.yml"), "test:\n  route: /a\n").unwrap();

        let result = discover_all(dir.path()).unwrap();
        assert_eq!(result.directory.files.len(), 2);
        assert_eq!(
            result
                .directory
                .files
                .iter()
                .map(|descriptor| {
                    descriptor
                        .file
                        .as_deref()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                })
                .collect::<Vec<_>>(),
            vec![Some("a.spec.yml"), Some("b.spec.yml")]
        );
    }

    #[test]
    fn discover_sorts_child_directories_by_path() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("z-child")).unwrap();
        fs::create_dir(dir.path().join("a-child")).unwrap();

        let result = discover_all(dir.path()).unwrap();
        assert_eq!(
            result
                .directory
                .children
                .iter()
                .filter_map(|child| child.dir.file_name().and_then(|name| name.to_str()))
                .collect::<Vec<_>>(),
            vec!["a-child", "z-child"]
        );
    }

    #[test]
    fn discover_finds_config_file_and_parses_options() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("test.config.yml"),
            "base_uri: http://localhost:9090\n",
        )
        .unwrap();

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
        assert!(result.directory.files.is_empty());
        assert!(result.directory.options.is_empty());
    }

    #[test]
    fn discover_ignores_yaml_files_without_recognized_suffix() {
        let dir = tempdir().unwrap();
        // "data.yml" has stem "data" — no .spec/.config/.template suffix, so ignored
        fs::write(dir.path().join("data.yml"), "key: value\n").unwrap();
        fs::write(dir.path().join("setup.yaml"), "key: value\n").unwrap();

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_for_test(
            dir.path(),
            None,
            &mut HashMap::new(),
            std::slice::from_ref(&sub),
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

        let result = discover_for_test(
            dir.path(),
            None,
            &mut HashMap::new(),
            &[sub.join("test.spec.yml")],
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
    fn discover_combines_file_and_directory_run_paths() {
        let dir = tempdir().unwrap();
        let selected_dir = dir.path().join("selected");
        let excluded_dir = dir.path().join("excluded");
        fs::create_dir(&selected_dir).unwrap();
        fs::create_dir(&excluded_dir).unwrap();

        let selected_file = dir.path().join("root.spec.yml");
        fs::write(&selected_file, "test:\n  route: /root\n").unwrap();
        fs::write(
            selected_dir.join("nested.spec.yml"),
            "test:\n  route: /nested\n",
        )
        .unwrap();
        fs::write(
            excluded_dir.join("excluded.spec.yml"),
            "test:\n  route: /excluded\n",
        )
        .unwrap();

        let result = discover_for_test(
            dir.path(),
            None,
            &mut HashMap::new(),
            &[selected_file, selected_dir],
        )
        .unwrap();

        assert_eq!(
            discovered_file_names(&result),
            vec!["nested.spec.yml", "root.spec.yml"]
        );
    }

    #[test]
    fn overlapping_run_paths_do_not_duplicate_specs() {
        let dir = tempdir().unwrap();
        let selected_dir = dir.path().join("selected");
        fs::create_dir(&selected_dir).unwrap();
        let selected_file = selected_dir.join("test.spec.yml");
        fs::write(&selected_file, "test:\n  route: /test\n").unwrap();

        let result = discover_for_test(
            dir.path(),
            None,
            &mut HashMap::new(),
            &[selected_dir, selected_file],
        )
        .unwrap();

        assert_eq!(discovered_file_names(&result), vec!["test.spec.yml"]);
    }

    #[test]
    fn discover_recurses_into_deeply_nested_directories() {
        let dir = tempdir().unwrap();
        let deep = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.spec.yml"), "test:\n  route: /deep\n").unwrap();

        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
        let child = &result.directory.children[0];
        assert_eq!(child.options.len(), 1);
        assert_eq!(child.options[0].base_uri.as_deref(), Some("http://parent"));
    }

    #[test]
    fn discover_uses_inherited_environment_as_the_first_template_layer() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".env"),
            "NAME={{ env.NAME }}-project\nURL=https://{{ env.HOST }}\n",
        )
        .unwrap();
        let mut inherited = HashMap::from([
            ("NAME".to_owned(), "cli".to_owned()),
            ("HOST".to_owned(), "api.example.com".to_owned()),
        ]);

        let result = discover_for_test(
            dir.path(),
            None,
            &mut inherited,
            &[dir.path().to_path_buf()],
        )
        .unwrap();

        assert_eq!(result.directory.envs["NAME"], "cli-project");
        assert_eq!(result.directory.envs["URL"], "https://api.example.com");
        assert_eq!(result.directory.envs["HOST"], "api.example.com");
    }

    #[test]
    fn discover_resolves_environment_files_in_sorted_file_order() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.env"), "GREETING=Hello\n").unwrap();
        fs::write(
            dir.path().join("b.env"),
            "GREETING={{ env.GREETING }}, World!\nRESULT={{ env.GREETING }}\n",
        )
        .unwrap();

        let result = discover_all(dir.path()).unwrap();

        assert_eq!(result.directory.envs["GREETING"], "Hello, World!");
        assert_eq!(result.directory.envs["RESULT"], "Hello, World!");
    }

    #[test]
    fn discover_resolves_parent_and_child_environment_cascades() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        fs::create_dir(&project).unwrap();
        fs::write(dir.path().join(".env"), "MY_VAR=Hello\n").unwrap();
        fs::write(
            project.join(".env"),
            "MY_VAR={{ env.MY_VAR }}, World!\nRESULT_VAR=Greetings! {{ env.MY_VAR }}\n",
        )
        .unwrap();

        let result = discover_all(dir.path()).unwrap();
        let child = &result.directory.children[0];

        assert_eq!(result.directory.envs["MY_VAR"], "Hello");
        assert!(!result.directory.envs.contains_key("RESULT_VAR"));
        assert_eq!(child.envs["MY_VAR"], "Hello, World!");
        assert_eq!(child.envs["RESULT_VAR"], "Greetings! Hello, World!");
    }

    #[test]
    fn discover_keeps_child_environment_changes_out_of_siblings() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("a-first");
        let second = dir.path().join("b-second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(dir.path().join(".env"), "SHARED=parent\n").unwrap();
        fs::write(
            first.join(".env"),
            "SHARED=first\nFIRST_ONLY=visible-in-first\n",
        )
        .unwrap();
        fs::write(second.join(".env"), "RESULT={{ env.SHARED }}\n").unwrap();

        let result = discover_all(dir.path()).unwrap();
        let first = &result.directory.children[0];
        let second = &result.directory.children[1];

        assert_eq!(result.directory.envs["SHARED"], "parent");
        assert!(!result.directory.envs.contains_key("FIRST_ONLY"));
        assert_eq!(first.envs["SHARED"], "first");
        assert_eq!(first.envs["FIRST_ONLY"], "visible-in-first");
        assert_eq!(second.envs["SHARED"], "parent");
        assert_eq!(second.envs["RESULT"], "parent");
        assert!(!second.envs.contains_key("FIRST_ONLY"));
    }

    #[test]
    fn discover_propagates_environment_template_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("broken.env"), "BROKEN={{ env.MISSING\n").unwrap();

        let error = discover_all(dir.path()).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("BROKEN"));
        assert!(message.contains("broken.env"));
        assert!(message.contains("line 1"));
    }

    #[test]
    fn discover_uses_provided_inherited_configs_as_starting_options() {
        let dir = tempdir().unwrap();
        let inherited = RunOptions {
            base_uri: Some("http://inherited".to_string()),
            ..RunOptions::default()
        };

        let result = discover_for_test(
            dir.path(),
            Some(vec![inherited]),
            &mut HashMap::new(),
            &[dir.path().to_path_buf()],
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

        let result = discover_all(dir.path()).unwrap();
        assert!(
            result.templates.contains_key("child"),
            "child templates should bubble up"
        );
    }

    #[test]
    fn discover_returns_error_for_nonexistent_directory() {
        let result = discover_for_test(
            &PathBuf::from("/no/such/directory/exists"),
            None,
            &mut HashMap::new(),
            &[PathBuf::new()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn discover_always_includes_builtin_templates_even_in_empty_dir() {
        let dir = tempdir().unwrap();
        let result = discover_all(dir.path()).unwrap();
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

        let result = discover_all(dir.path()).unwrap();
        assert!(result.templates.contains_key("console"));
        assert!(result.templates.contains_key("myreport"));
    }
}
