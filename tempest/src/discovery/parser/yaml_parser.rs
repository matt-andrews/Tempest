use crate::discovery::BUILTIN_REPORTERS;
use crate::discovery::parser::FileParser;
use crate::models::descriptor::Descriptor;
use crate::models::project::Project;
use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use anyhow::Context;
use include_dir::{Dir, File};
use liquid_core::model::DateTime;
use std::fs;
use std::path::{Path, PathBuf};

pub struct YamlFileParser;
impl FileParser for YamlFileParser {
    fn parse_descriptor(&self, path: &Path) -> anyhow::Result<Descriptor> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read YAML file {}", path.display()))?;
        let mut result: Descriptor = noyalib::from_str(&contents)
            .with_context(|| format!("invalid YAML in {}", path.display()))?;
        Self::validate_descriptor(&result)
            .with_context(|| format!("invalid descriptor in {}", path.display()))?;
        Self::assign_file(&mut result, path);
        Ok(result)
    }

    fn parse_config(&self, path: &Path) -> anyhow::Result<RunOptions> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read YAML file {}", path.display()))?;
        let config: RunOptions = noyalib::from_str(&contents)
            .with_context(|| format!("invalid YAML in {}", path.display()))?;
        if config.loop_count.is_some() {
            anyhow::bail!(
                "`loop` is only valid under a descriptor's `options` in {}",
                path.display()
            );
        }
        Ok(config)
    }

    fn parse_report_template(&self, path: &Path) -> anyhow::Result<ReportTemplate> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read YAML file {}", path.display()))?;
        let mut template: ReportTemplate = noyalib::from_str(&contents)
            .with_context(|| format!("invalid YAML in {}", path.display()))?;

        let parent = path.parent().map(PathBuf::from).unwrap_or_default();

        template.test_template = Self::resolve_liquid_ref(template.test_template, &parent);
        template.section_template = Self::resolve_liquid_ref(template.section_template, &parent);
        template.error_template = Self::resolve_liquid_ref(template.error_template, &parent);
        template.debug_template = Self::resolve_liquid_ref(template.debug_template, &parent);
        template.title_template = Self::resolve_liquid_ref(template.title_template, &parent);
        template.summary_template = Self::resolve_liquid_ref(template.summary_template, &parent);

        Ok(template)
    }

    fn parse_embedded_report_template(
        &self,
        file: &File,
        dir: &Dir,
    ) -> anyhow::Result<ReportTemplate> {
        let contents = file.contents_utf8().unwrap_or_default();
        let mut template = noyalib::from_str::<ReportTemplate>(contents)
            .with_context(|| format!("invalid embedded YAML in {}", file.path().display()))?;

        template.test_template = Self::resolve_embedded_liquid(template.test_template, dir);
        template.section_template = Self::resolve_embedded_liquid(template.section_template, dir);
        template.error_template = Self::resolve_embedded_liquid(template.error_template, dir);
        template.debug_template = Self::resolve_embedded_liquid(template.debug_template, dir);
        template.title_template = Self::resolve_embedded_liquid(template.title_template, dir);
        template.summary_template = Self::resolve_embedded_liquid(template.summary_template, dir);

        Ok(template)
    }

    fn parse_project(&self, path: &Path) -> anyhow::Result<Project> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("could not read YAML file {}", path.display()))?;
        let mut result: Project = noyalib::from_str(&contents)
            .with_context(|| format!("invalid YAML in {}", path.display()))?;

        //always get start time
        result.options = Some(
            RunOptions {
                start_time: Some(DateTime::now()),
                ..RunOptions::default()
            }
            .merge(result.options.unwrap_or_default()),
        );

        Ok(result)
    }
}

impl YamlFileParser {
    fn validate_descriptor(descriptor: &Descriptor) -> anyhow::Result<()> {
        if descriptor.profiles.as_ref().is_some_and(Vec::is_empty) {
            anyhow::bail!("`profiles` must contain at least one profile");
        }

        for child in descriptor.describe.as_deref().unwrap_or_default() {
            Self::validate_descriptor(child)?;
        }

        Ok(())
    }

    fn assign_file(descriptor: &mut Descriptor, path: &Path) {
        descriptor.file = Some(path.to_path_buf());

        for child in descriptor.describe.iter_mut().flatten() {
            Self::assign_file(child, path);
        }
    }

    fn resolve_liquid_ref(value: Option<String>, base_dir: &Path) -> Option<String> {
        value.map(|v| {
            let trimmed = v.trim();
            if trimmed.ends_with(".liquid") {
                let file_path = base_dir.join(trimmed);
                fs::read_to_string(&file_path)
                    .unwrap_or_else(|e| format!("<!-- could not load {trimmed}: {e} -->"))
            } else {
                v
            }
        })
    }
    fn resolve_embedded_liquid(value: Option<String>, dir: &Dir) -> Option<String> {
        value.map(|v| {
            let trimmed = v.trim();
            if !trimmed.ends_with(".liquid") {
                return v;
            }
            let liquid_path = dir.path().join(trimmed);
            BUILTIN_REPORTERS
                .get_file(&liquid_path)
                .and_then(|f| f.contents_utf8())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("<!-- could not load embedded {trimmed} -->"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::BUILTIN_REPORTERS;
    use crate::discovery::parser::FileParser;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_descriptor_minimal_spec_with_route() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.spec.yml");
        fs::write(&path, "test:\n  route: /api/health\n").unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        assert_eq!(result.test.unwrap().route, "/api/health");
    }

    #[test]
    fn parse_descriptor_parses_all_top_level_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.spec.yml");
        let yaml = "name: Auth Suite\ndescription: Auth tests\ntags:\n  - auth\n  - api\ntest:\n  route: /api/login\n  verb: POST\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        assert_eq!(result.name.as_deref(), Some("Auth Suite"));
        assert_eq!(result.description.as_deref(), Some("Auth tests"));
        assert_eq!(result.tags.as_ref().unwrap(), &vec!["auth", "api"]);
        let test = result.test.unwrap();
        assert_eq!(test.route, "/api/login");
        assert_eq!(test.verb.as_deref(), Some("POST"));
    }

    #[test]
    fn parse_descriptor_preserves_let_declaration_order() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("let.spec.yml");
        let yaml =
            "test:\n  route: /items\n  let:\n    json: body.json()\n    item: let.json.item\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        let declarations = result.test.unwrap().lets.unwrap();
        let names = declarations.keys().map(String::as_str).collect::<Vec<_>>();

        assert_eq!(names, vec!["json", "item"]);
    }

    #[test]
    fn parse_descriptor_parses_nested_describe_blocks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested.spec.yml");
        let yaml = "name: Suite\ndescribe:\n  - name: Child A\n    test:\n      route: /a\n  - name: Child B\n    test:\n      route: /b\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        let children = result.describe.unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name.as_deref(), Some("Child A"));
        assert_eq!(children[1].name.as_deref(), Some("Child B"));
    }

    #[test]
    fn parse_descriptor_preserves_options_block() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.spec.yml");
        let yaml =
            "test:\n  route: /api\noptions:\n  base_uri: http://localhost:8080\n  debug: true\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        let opts = result.options.unwrap();
        assert_eq!(opts.base_uri.as_deref(), Some("http://localhost:8080"));
        assert_eq!(opts.debug, Some(true));
    }

    #[test]
    fn parse_descriptor_parses_profiles_and_local_loop() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("expanded.spec.yml");
        let yaml = "profiles:\n  - region: us\n    enabled: true\n  - region: eu\n    enabled: false\noptions:\n  loop: 2\ntest:\n  route: /health\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        let profiles = result.profiles.unwrap();

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0]["region"], serde_json::json!("us"));
        assert_eq!(profiles[0]["enabled"], serde_json::json!(true));
        assert_eq!(result.options.unwrap().loop_count.unwrap().get(), 2);
    }

    #[test]
    fn parse_descriptor_rejects_empty_profiles() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty-profiles.spec.yml");
        fs::write(&path, "profiles: []\ntest:\n  route: /health\n").unwrap();

        let error = YamlFileParser.parse_descriptor(&path).unwrap_err();

        assert!(format!("{error:#}").contains("must contain at least one profile"));
    }

    #[test]
    fn parse_descriptor_rejects_zero_loop_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("zero-loop.spec.yml");
        fs::write(&path, "options:\n  loop: 0\ntest:\n  route: /health\n").unwrap();

        assert!(YamlFileParser.parse_descriptor(&path).is_err());
    }

    #[test]
    fn parse_config_rejects_descriptor_loop_option() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("loop.config.yml");
        fs::write(&path, "loop: 2\n").unwrap();

        let error = YamlFileParser.parse_config(&path).unwrap_err();

        assert!(format!("{error:#}").contains("only valid under a descriptor"));
    }

    #[test]
    fn parse_descriptor_returns_empty_for_invalid_yaml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.spec.yml");
        fs::write(&path, ":: not valid yaml ::").unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        assert!(result.name.is_none());
        assert!(result.test.is_none());
        assert!(result.describe.is_none());
    }

    #[test]
    fn parse_descriptor_returns_error_for_missing_file() {
        let result = YamlFileParser.parse_descriptor(&PathBuf::from("/nonexistent/path/test.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn parse_descriptor_all_fields_optional_empty_doc_parses() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.spec.yml");
        fs::write(&path, "{}\n").unwrap();

        let result = YamlFileParser.parse_descriptor(&path).unwrap();
        assert!(result.name.is_none());
        assert!(result.test.is_none());
        assert!(result.describe.is_none());
    }

    #[test]
    fn parse_config_parses_base_uri_and_debug() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.config.yml");
        fs::write(&path, "base_uri: http://localhost:3000\ndebug: true\n").unwrap();

        let result = YamlFileParser.parse_config(&path).unwrap();
        assert_eq!(result.base_uri.as_deref(), Some("http://localhost:3000"));
        assert_eq!(result.debug, Some(true));
    }

    #[test]
    fn parse_config_parses_reports_list() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.config.yml");
        fs::write(&path, "reports:\n  - console\n  - html\n").unwrap();

        let result = YamlFileParser.parse_config(&path).unwrap();
        assert_eq!(result.reports.as_ref().unwrap(), &vec!["console", "html"]);
    }

    #[test]
    fn parse_config_parses_file_concurrency() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.config.yml");
        fs::write(&path, "concurrent: true\n").unwrap();

        let result = YamlFileParser.parse_config(&path).unwrap();
        assert_eq!(result.concurrent, Some(true));
    }

    #[test]
    fn parse_config_partial_fields_leave_others_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.config.yml");
        fs::write(&path, "base_uri: http://example.com\n").unwrap();

        let result = YamlFileParser.parse_config(&path).unwrap();
        assert_eq!(result.base_uri.as_deref(), Some("http://example.com"));
        assert!(result.debug.is_none());
        assert!(result.reports.is_none());
        assert!(result.concurrent.is_none());
    }

    #[test]
    fn parse_config_empty_doc_gives_all_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.config.yml");
        fs::write(&path, "{}\n").unwrap();

        let result = YamlFileParser.parse_config(&path).unwrap();
        assert!(result.base_uri.is_none());
        assert!(result.debug.is_none());
        assert!(result.reports.is_none());
        assert!(result.concurrent.is_none());
    }

    #[test]
    fn parse_config_returns_error_for_invalid_yaml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.config.yml");
        fs::write(&path, ": bad: yaml: here").unwrap();

        let error = YamlFileParser.parse_config(&path).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("invalid YAML"));
        assert!(message.contains("bad.config.yml"));
    }

    #[test]
    fn parse_config_returns_error_for_missing_file() {
        let result = YamlFileParser.parse_config(&PathBuf::from("/no/such/file.yml"));
        let message = format!("{:#}", result.unwrap_err());
        assert!(message.contains("could not read YAML file"));
        assert!(message.contains("file.yml"));
    }

    #[test]
    fn parse_project_parses_fields_options_and_assigns_start_time() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("example.project.yml");
        fs::write(
            &path,
            r#"name: Example Project
version: '2'
env:
  HOST: api.example.com
include:
  - smoke.spec.yml
  - api
warn_as_err: true
success_exit: 0
flaky_exit: 2
failed_exit: 1
options:
  base_uri: https://api.example.com
  retries: 3
"#,
        )
        .unwrap();

        let project = YamlFileParser.parse_project(&path).unwrap();

        assert_eq!(project.name, "Example Project");
        assert_eq!(project.version.as_deref(), Some("2"));
        assert_eq!(project.env.as_ref().unwrap()["HOST"], "api.example.com");
        assert_eq!(
            project.include.as_deref(),
            Some([PathBuf::from("smoke.spec.yml"), PathBuf::from("api")].as_slice())
        );
        assert_eq!(project.warn_as_err, Some(true));
        assert_eq!(project.success_exit, Some(0));
        assert_eq!(project.flaky_exit, Some(2));
        assert_eq!(project.failed_exit, Some(1));

        let options = project.options.unwrap();
        assert_eq!(options.base_uri.as_deref(), Some("https://api.example.com"));
        assert_eq!(options.retries, Some(3));
        assert!(options.start_time.is_some());
    }

    #[test]
    fn parse_project_returns_error_for_invalid_yaml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("invalid.project.yml");
        fs::write(&path, "name:\n  - not\n  - a string\n").unwrap();

        let error = YamlFileParser.parse_project(&path).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("invalid YAML"));
        assert!(message.contains("invalid.project.yml"));
    }

    #[test]
    fn parse_project_returns_error_for_missing_file() {
        let path = PathBuf::from("/no/such/project.yml");

        let error = YamlFileParser.parse_project(&path).unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("could not read YAML file"));
        assert!(message.contains("project.yml"));
    }

    #[test]
    fn parse_report_template_inline_strings_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("my.template.yml");
        let yaml = "test_template: \"{{ test.name }}\"\nsection_template: \"## {{ section }}\"\nerror_template: ERROR\ntitle_template: \"# Title\"\nsummary_template: Done\n";
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert_eq!(result.test_template.as_deref(), Some("{{ test.name }}"));
        assert_eq!(result.section_template.as_deref(), Some("## {{ section }}"));
        assert_eq!(result.error_template.as_deref(), Some("ERROR"));
        assert_eq!(result.title_template.as_deref(), Some("# Title"));
        assert_eq!(result.summary_template.as_deref(), Some("Done"));
    }

    #[test]
    fn parse_report_template_resolves_liquid_file_reference() {
        let dir = tempdir().unwrap();
        let liquid_content = "<p>{{ test.name }}</p>";
        fs::write(dir.path().join("test.liquid"), liquid_content).unwrap();

        let path = dir.path().join("my.template.yml");
        fs::write(&path, "test_template: test.liquid\n").unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert_eq!(result.test_template.as_deref(), Some(liquid_content));
    }

    #[test]
    fn parse_report_template_resolves_all_liquid_references() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.liquid"), "test content").unwrap();
        fs::write(dir.path().join("section.liquid"), "section content").unwrap();
        fs::write(dir.path().join("error.liquid"), "error content").unwrap();
        fs::write(dir.path().join("debug.liquid"), "debug content").unwrap();
        fs::write(dir.path().join("title.liquid"), "title content").unwrap();
        fs::write(dir.path().join("summary.liquid"), "summary content").unwrap();

        let yaml = "test_template: test.liquid\nsection_template: section.liquid\nerror_template: error.liquid\ndebug_template: debug.liquid\ntitle_template: title.liquid\nsummary_template: summary.liquid\n";
        let path = dir.path().join("all.template.yml");
        fs::write(&path, yaml).unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert_eq!(result.test_template.as_deref(), Some("test content"));
        assert_eq!(result.section_template.as_deref(), Some("section content"));
        assert_eq!(result.error_template.as_deref(), Some("error content"));
        assert_eq!(result.debug_template.as_deref(), Some("debug content"));
        assert_eq!(result.title_template.as_deref(), Some("title content"));
        assert_eq!(result.summary_template.as_deref(), Some("summary content"));
    }

    #[test]
    fn parse_report_template_embeds_error_for_missing_liquid_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("my.template.yml");
        fs::write(&path, "test_template: missing.liquid\n").unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        let tpl = result.test_template.unwrap();
        assert!(
            tpl.contains("could not load"),
            "should embed an error comment"
        );
        assert!(tpl.contains("missing.liquid"));
    }

    #[test]
    fn parse_report_template_none_fields_remain_none() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("minimal.template.yml");
        fs::write(&path, "{}\n").unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert!(result.test_template.is_none());
        assert!(result.section_template.is_none());
        assert!(result.error_template.is_none());
        assert!(result.debug_template.is_none());
        assert!(result.title_template.is_none());
        assert!(result.summary_template.is_none());
    }

    #[test]
    fn parse_report_template_non_liquid_string_not_treated_as_file_ref() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("my.template.yml");
        fs::write(&path, "test_template: just some inline content\n").unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert_eq!(
            result.test_template.as_deref(),
            Some("just some inline content")
        );
    }

    #[test]
    fn parse_report_template_returns_empty_for_invalid_yaml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.template.yml");
        fs::write(&path, ": invalid :").unwrap();

        let result = YamlFileParser.parse_report_template(&path).unwrap();
        assert!(result.test_template.is_none());
        assert!(result.section_template.is_none());
        assert!(result.error_template.is_none());
        assert!(result.debug_template.is_none());
        assert!(result.title_template.is_none());
        assert!(result.summary_template.is_none());
    }

    #[test]
    fn parse_embedded_report_template_resolves_console_liquid_refs() {
        let console_dir = BUILTIN_REPORTERS
            .get_dir("console_reporter")
            .expect("console_reporter should be embedded");

        let template_file = console_dir
            .files()
            .find(|f| f.path().file_name().and_then(|n| n.to_str()) == Some("console.template.yml"))
            .expect("console.template.yml should be embedded");

        let result = YamlFileParser
            .parse_embedded_report_template(template_file, console_dir)
            .unwrap();

        assert!(result.test_template.is_some());
        let tpl = result.test_template.as_deref().unwrap();
        assert!(
            !tpl.ends_with(".liquid"),
            "liquid ref should be resolved to content, not left as filename"
        );
        assert!(
            !tpl.contains("could not load"),
            "embedded liquid file should resolve successfully"
        );
    }

    #[test]
    fn parse_embedded_report_template_all_fields_resolved() {
        let console_dir = BUILTIN_REPORTERS
            .get_dir("console_reporter")
            .expect("console_reporter should be embedded");

        let template_file = console_dir
            .files()
            .find(|f| f.path().file_name().and_then(|n| n.to_str()) == Some("console.template.yml"))
            .expect("console.template.yml should be embedded");

        let result = YamlFileParser
            .parse_embedded_report_template(template_file, console_dir)
            .unwrap();

        for (name, field) in [
            ("test_template", &result.test_template),
            ("section_template", &result.section_template),
            ("error_template", &result.error_template),
            ("debug_template", &result.debug_template),
            ("title_template", &result.title_template),
            ("summary_template", &result.summary_template),
        ] {
            let content = field
                .as_deref()
                .unwrap_or_else(|| panic!("{name} should be Some"));
            assert!(
                !content.ends_with(".liquid"),
                "{name} should be resolved content, not a filename"
            );
        }
    }
}
