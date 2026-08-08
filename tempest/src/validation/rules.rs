use crate::discovery::DiscoveryResult;
use crate::models::descriptor::Descriptor;
use crate::models::directory_node::DirectoryNode;
use crate::models::run_options::RunOptions;
use crate::models::templated::Templated;
use crate::models::test_spec::TestSpec;
use crate::templating::liquid::LiquidEngine;
use crate::validation::validation_diagnostic::ValidationDiagnostic;
use crate::validation::validation_report::ValidationReport;
use cel_interpreter::Program;
use serde_json::Value as JsonValue;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

const CEL_INVALID: &str = "cel.invalid";
const DESCRIPTOR_EMPTY: &str = "descriptor.empty";
const LIQUID_INVALID: &str = "liquid.invalid";
const PROJECT_NO_TESTS: &str = "project.no-tests";
const REPORT_INVALID_LIQUID: &str = "report.invalid-liquid";
const REPORT_UNKNOWN: &str = "report.unknown";
const TEST_EMPTY_ROUTE: &str = "test.empty-route";
const TEST_UNSUPPORTED_VERB: &str = "test.unsupported-verb";

pub fn run(project: &DiscoveryResult) -> ValidationReport {
    let mut diagnostics = Vec::new();

    project_no_tests(project, &mut diagnostics);
    walk_directory(
        project,
        &project.directory,
        0,
        &LiquidEngine,
        &mut diagnostics,
    );
    report_templates_have_valid_liquid(project, &LiquidEngine, &mut diagnostics);

    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.context.cmp(&right.context))
            .then_with(|| left.code.cmp(right.code))
            .then_with(|| left.message.cmp(&right.message))
    });

    ValidationReport {
        diagnostics,
        checked_specs: project
            .directory
            .walk()
            .map(|directory| directory.files.len())
            .sum(),
    }
}

fn walk_directory(
    project: &DiscoveryResult,
    directory: &DirectoryNode,
    inherited_option_count: usize,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let local_options = directory
        .options
        .get(inherited_option_count..)
        .unwrap_or_default();

    for options in local_options {
        let path = Some(directory.dir.clone());
        let context = Some("Directory config".to_owned());
        run_options_have_valid_liquid(options, path.clone(), context.clone(), liquid, diagnostics);
        reports_are_known(project, options, path, context, diagnostics);
    }

    for root in &directory.files {
        for (descriptor, _ancestor_options, title_path) in root.descendants() {
            descriptor_empty(project, descriptor, &title_path, diagnostics);
            descriptor_has_valid_liquid(project, descriptor, &title_path, liquid, diagnostics);

            if let Some(options) = &descriptor.options {
                let path = descriptor_path(project, descriptor);
                let context = descriptor_field_context(descriptor, &title_path, "options");
                run_options_have_valid_liquid(
                    options,
                    path.clone(),
                    context.clone(),
                    liquid,
                    diagnostics,
                );
                reports_are_known(project, options, path, context, diagnostics);
            }

            if let Some(test) = &descriptor.test {
                test_empty_route(project, descriptor, test, &title_path, diagnostics);
                test_unsupported_verb(project, descriptor, test, &title_path, diagnostics);
                test_has_valid_liquid(project, descriptor, test, &title_path, liquid, diagnostics);
                test_has_valid_cel(project, descriptor, test, &title_path, diagnostics);
            }
        }
    }

    for child in &directory.children {
        walk_directory(project, child, directory.options.len(), liquid, diagnostics);
    }
}

fn project_no_tests(project: &DiscoveryResult, diagnostics: &mut Vec<ValidationDiagnostic>) {
    if !project.directory.has_tests() {
        diagnostics.push(ValidationDiagnostic::error(
            PROJECT_NO_TESTS,
            Some(project.directory.dir.clone()),
            None,
            "No selected runnable tests were discovered",
        ));
    }
}

fn descriptor_empty(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    title_path: &[String],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let has_children = descriptor
        .describe
        .as_deref()
        .is_some_and(|children| !children.is_empty());

    if descriptor.test.is_none() && !has_children {
        diagnostics.push(ValidationDiagnostic::error(
            DESCRIPTOR_EMPTY,
            descriptor_path(project, descriptor),
            descriptor_context(descriptor, title_path),
            "A terminal descriptor has neither test nor children",
        ));
    }
}

fn test_empty_route(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    test: &TestSpec,
    title_path: &[String],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if test.route.trim().is_empty() {
        diagnostics.push(ValidationDiagnostic::error(
            TEST_EMPTY_ROUTE,
            descriptor_path(project, descriptor),
            descriptor_field_context(descriptor, title_path, "test.route"),
            "Route is blank",
        ));
    }
}

fn test_unsupported_verb(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    test: &TestSpec,
    title_path: &[String],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let Some(verb) = test.verb.as_deref() else {
        return;
    };

    if contains_liquid_markup(verb) {
        return;
    }

    const SUPPORTED_VERBS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];
    let normalized = verb.to_ascii_uppercase();

    if !SUPPORTED_VERBS.contains(&normalized.as_str()) {
        diagnostics.push(ValidationDiagnostic::error(
            TEST_UNSUPPORTED_VERB,
            descriptor_path(project, descriptor),
            descriptor_field_context(descriptor, title_path, "test.verb"),
            format!("Unsupported HTTP verb `{verb}`"),
        ));
    }
}

fn test_has_valid_cel(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    test: &TestSpec,
    title_path: &[String],
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    for (index, expression) in test
        .assert
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        validate_cel_expression(
            project,
            descriptor,
            title_path,
            &format!("test.assert[{index}]"),
            expression,
            diagnostics,
        );
    }

    for (name, expression) in test.lets.as_ref().into_iter().flatten() {
        validate_cel_expression(
            project,
            descriptor,
            title_path,
            &format!("test.let.{name}"),
            expression,
            diagnostics,
        );
    }

    for (name, expression) in test.vars.as_ref().into_iter().flatten() {
        validate_cel_expression(
            project,
            descriptor,
            title_path,
            &format!("test.vars.{name}"),
            expression,
            diagnostics,
        );
    }
}

fn validate_cel_expression(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    title_path: &[String],
    field: &str,
    expression: &str,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    // Liquid is rendered before CEL is compiled at runtime. Its final CEL source
    // cannot be known during validation, so only validate the Liquid syntax here.
    if contains_liquid_markup(expression) {
        return;
    }

    let error = match catch_unwind(AssertUnwindSafe(|| Program::compile(expression))) {
        Ok(Ok(_)) => return,
        Ok(Err(error)) => error.to_string(),
        Err(_) => "the CEL parser could not parse this expression".to_owned(),
    };

    diagnostics.push(ValidationDiagnostic::error(
        CEL_INVALID,
        descriptor_path(project, descriptor),
        descriptor_field_context(descriptor, title_path, field),
        format!("Invalid CEL expression: {error}"),
    ));
}

fn descriptor_has_valid_liquid(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    title_path: &[String],
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let path = descriptor_path(project, descriptor);

    if let Some(name) = &descriptor.name {
        validate_liquid(
            LIQUID_INVALID,
            name,
            path.clone(),
            descriptor_field_context(descriptor, title_path, "name"),
            liquid,
            diagnostics,
        );
    }
    if let Some(description) = &descriptor.description {
        validate_liquid(
            LIQUID_INVALID,
            description,
            path.clone(),
            descriptor_field_context(descriptor, title_path, "description"),
            liquid,
            diagnostics,
        );
    }
    for (index, tag) in descriptor
        .tags
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        validate_liquid(
            LIQUID_INVALID,
            tag,
            path.clone(),
            descriptor_field_context(descriptor, title_path, &format!("tags[{index}]")),
            liquid,
            diagnostics,
        );
    }

    for (profile_index, profile) in descriptor
        .profiles
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        for (key, value) in profile {
            validate_liquid(
                LIQUID_INVALID,
                key,
                path.clone(),
                descriptor_field_context(
                    descriptor,
                    title_path,
                    &format!("profiles[{profile_index}].key"),
                ),
                liquid,
                diagnostics,
            );
            validate_profile_liquid(
                project,
                descriptor,
                title_path,
                &format!("profiles[{profile_index}].{key}"),
                value,
                liquid,
                diagnostics,
            );
        }
    }
}

fn validate_profile_liquid(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    title_path: &[String],
    field: &str,
    value: &JsonValue,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match value {
        JsonValue::String(value) => validate_liquid(
            LIQUID_INVALID,
            value,
            descriptor_path(project, descriptor),
            descriptor_field_context(descriptor, title_path, field),
            liquid,
            diagnostics,
        ),
        JsonValue::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_profile_liquid(
                    project,
                    descriptor,
                    title_path,
                    &format!("{field}[{index}]"),
                    value,
                    liquid,
                    diagnostics,
                );
            }
        }
        JsonValue::Object(values) => {
            for (key, value) in values {
                validate_liquid(
                    LIQUID_INVALID,
                    key,
                    descriptor_path(project, descriptor),
                    descriptor_field_context(descriptor, title_path, &format!("{field}.key")),
                    liquid,
                    diagnostics,
                );
                validate_profile_liquid(
                    project,
                    descriptor,
                    title_path,
                    &format!("{field}.{key}"),
                    value,
                    liquid,
                    diagnostics,
                );
            }
        }
        _ => {}
    }
}

fn test_has_valid_liquid(
    project: &DiscoveryResult,
    descriptor: &Descriptor,
    test: &TestSpec,
    title_path: &[String],
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let path = descriptor_path(project, descriptor);
    validate_liquid(
        LIQUID_INVALID,
        &test.route,
        path.clone(),
        descriptor_field_context(descriptor, title_path, "test.route"),
        liquid,
        diagnostics,
    );

    if let Some(verb) = &test.verb {
        validate_liquid(
            LIQUID_INVALID,
            verb,
            path.clone(),
            descriptor_field_context(descriptor, title_path, "test.verb"),
            liquid,
            diagnostics,
        );
    }
    if let Some(body) = &test.body {
        validate_liquid(
            LIQUID_INVALID,
            body,
            path.clone(),
            descriptor_field_context(descriptor, title_path, "test.body"),
            liquid,
            diagnostics,
        );
    }

    for (index, expression) in test
        .assert
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        validate_liquid(
            LIQUID_INVALID,
            expression,
            path.clone(),
            descriptor_field_context(descriptor, title_path, &format!("test.assert[{index}]")),
            liquid,
            diagnostics,
        );
    }

    validate_string_map_liquid(
        test.vars.as_ref(),
        "test.vars",
        descriptor,
        title_path,
        path.clone(),
        liquid,
        diagnostics,
    );
    validate_string_map_liquid(
        test.query.as_ref(),
        "test.query",
        descriptor,
        title_path,
        path.clone(),
        liquid,
        diagnostics,
    );
    validate_string_map_liquid(
        test.headers.as_ref(),
        "test.headers",
        descriptor,
        title_path,
        path.clone(),
        liquid,
        diagnostics,
    );

    if let Some(lets) = &test.lets {
        for (name, expression) in lets {
            validate_liquid(
                LIQUID_INVALID,
                name,
                path.clone(),
                descriptor_field_context(descriptor, title_path, "test.let.key"),
                liquid,
                diagnostics,
            );
            validate_liquid(
                LIQUID_INVALID,
                expression,
                path.clone(),
                descriptor_field_context(descriptor, title_path, &format!("test.let.{name}")),
                liquid,
                diagnostics,
            );
        }
    }
}

fn validate_string_map_liquid(
    values: Option<&std::collections::HashMap<String, String>>,
    field: &str,
    descriptor: &Descriptor,
    title_path: &[String],
    path: Option<PathBuf>,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    for (key, value) in values.into_iter().flatten() {
        validate_liquid(
            LIQUID_INVALID,
            key,
            path.clone(),
            descriptor_field_context(descriptor, title_path, &format!("{field}.key")),
            liquid,
            diagnostics,
        );
        validate_liquid(
            LIQUID_INVALID,
            value,
            path.clone(),
            descriptor_field_context(descriptor, title_path, &format!("{field}.{key}")),
            liquid,
            diagnostics,
        );
    }
}

fn run_options_have_valid_liquid(
    options: &RunOptions,
    path: Option<PathBuf>,
    context: Option<String>,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if let Some(base_uri) = &options.base_uri {
        validate_liquid(
            LIQUID_INVALID,
            base_uri,
            path.clone(),
            append_field_context(context.as_deref(), "base_uri"),
            liquid,
            diagnostics,
        );
    }
    if let Some(Templated::Liquid(skip)) = &options.skip {
        validate_liquid(
            LIQUID_INVALID,
            skip,
            path,
            append_field_context(context.as_deref(), "skip"),
            liquid,
            diagnostics,
        );
    }
}

fn reports_are_known(
    project: &DiscoveryResult,
    options: &RunOptions,
    path: Option<PathBuf>,
    context: Option<String>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    for (index, report) in options
        .reports
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        if !project.templates.contains_key(report) {
            diagnostics.push(ValidationDiagnostic::error(
                REPORT_UNKNOWN,
                path.clone(),
                append_field_context(context.as_deref(), &format!("reports[{index}]")),
                format!("Unknown report `{report}`"),
            ));
        }
    }
}

fn report_templates_have_valid_liquid(
    project: &DiscoveryResult,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    for (name, template) in &project.templates {
        validate_report_template_field(
            project,
            name,
            "test_template",
            template.test_template.as_deref(),
            liquid,
            diagnostics,
        );
        validate_report_template_field(
            project,
            name,
            "section_template",
            template.section_template.as_deref(),
            liquid,
            diagnostics,
        );
        validate_report_template_field(
            project,
            name,
            "error_template",
            template.error_template.as_deref(),
            liquid,
            diagnostics,
        );
        validate_report_template_field(
            project,
            name,
            "debug_template",
            template.debug_template.as_deref(),
            liquid,
            diagnostics,
        );
        validate_report_template_field(
            project,
            name,
            "title_template",
            template.title_template.as_deref(),
            liquid,
            diagnostics,
        );
        validate_report_template_field(
            project,
            name,
            "summary_template",
            template.summary_template.as_deref(),
            liquid,
            diagnostics,
        );

        if let Some(file_name) = template
            .file
            .as_ref()
            .and_then(|file| file.file_name.as_deref())
        {
            validate_liquid(
                REPORT_INVALID_LIQUID,
                file_name,
                Some(project.directory.dir.clone()),
                Some(format!("Report: {name} | file.file_name")),
                liquid,
                diagnostics,
            );
        }
    }
}

fn validate_report_template_field(
    project: &DiscoveryResult,
    name: &str,
    field: &str,
    source: Option<&str>,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if let Some(source) = source {
        validate_liquid(
            REPORT_INVALID_LIQUID,
            source,
            Some(project.directory.dir.clone()),
            Some(format!("Report: {name} | {field}")),
            liquid,
            diagnostics,
        );
    }
}

fn validate_liquid(
    code: &'static str,
    source: &str,
    path: Option<PathBuf>,
    context: Option<String>,
    liquid: &LiquidEngine,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if let Err(error) = liquid.validate_syntax(source) {
        diagnostics.push(ValidationDiagnostic::error(
            code,
            path,
            context,
            format!("Invalid Liquid syntax: {error}"),
        ));
    }
}

fn contains_liquid_markup(source: &str) -> bool {
    source.contains("{{") || source.contains("{%")
}

fn descriptor_path(project: &DiscoveryResult, descriptor: &Descriptor) -> Option<PathBuf> {
    descriptor
        .file
        .clone()
        .or_else(|| Some(project.directory.dir.clone()))
}

fn descriptor_context(descriptor: &Descriptor, title_path: &[String]) -> Option<String> {
    let mut parts = title_path.to_vec();

    if let Some(name) = descriptor
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
    {
        parts.push(name.to_owned());
    }

    (!parts.is_empty()).then(|| parts.join(" > "))
}

fn descriptor_field_context(
    descriptor: &Descriptor,
    title_path: &[String],
    field: &str,
) -> Option<String> {
    append_field_context(descriptor_context(descriptor, title_path).as_deref(), field)
}

fn append_field_context(context: Option<&str>, field: &str) -> Option<String> {
    Some(match context {
        Some(context) if !context.is_empty() => format!("{context} | {field}"),
        _ => field.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn discover_files(files: &[(&str, &str)]) -> (TempDir, DiscoveryResult) {
        let directory = tempfile::tempdir().unwrap();
        for (name, contents) in files {
            let path = directory.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        let mut envs = HashMap::new();
        let project = discovery::discover(
            directory.path(),
            None,
            &mut envs,
            &[directory.path().to_path_buf()],
            &LiquidEngine,
        )
        .unwrap();

        (directory, project)
    }

    fn diagnostics_with_code<'a>(
        report: &'a ValidationReport,
        code: &str,
    ) -> Vec<&'a ValidationDiagnostic> {
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .collect()
    }

    #[test]
    fn invalid_static_cel_is_reported_for_assert_let_and_vars() {
        let (_directory, project) = discover_files(&[(
            "invalid.spec.yml",
            "test:\n  route: https://example.com\n  assert:\n    - 'status =='\n  let:\n    broken: '('\n  vars:\n    broken: '['\n",
        )]);

        let report = run(&project);
        let diagnostics = diagnostics_with_code(&report, CEL_INVALID);

        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn cel_with_liquid_is_deferred_until_runtime() {
        let (_directory, project) = discover_files(&[(
            "templated.spec.yml",
            "profiles:\n  - expected: 200\ntest:\n  route: https://example.com\n  assert:\n    - 'status == {{ profile.expected }}'\n",
        )]);

        let report = run(&project);

        assert!(diagnostics_with_code(&report, CEL_INVALID).is_empty());
        assert!(diagnostics_with_code(&report, LIQUID_INVALID).is_empty());
    }

    #[test]
    fn invalid_liquid_in_spec_input_is_reported() {
        let (_directory, project) =
            discover_files(&[("invalid.spec.yml", "test:\n  route: '{{ broken'\n")]);

        let report = run(&project);
        let diagnostics = diagnostics_with_code(&report, LIQUID_INVALID);

        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .context
                .as_deref()
                .unwrap()
                .contains("test.route")
        );
    }

    #[test]
    fn unknown_reports_are_reported_once_at_each_declaration() {
        let (_directory, project) = discover_files(&[
            ("root.config.yml", "reports:\n  - missing-config\n"),
            (
                "nested/test.spec.yml",
                "options:\n  reports:\n    - missing-descriptor\ntest:\n  route: https://example.com\n",
            ),
        ]);

        let report = run(&project);
        let diagnostics = diagnostics_with_code(&report, REPORT_UNKNOWN);

        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("missing-config"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("missing-descriptor"))
        );
    }

    #[test]
    fn invalid_liquid_in_report_template_is_reported() {
        let (_directory, project) = discover_files(&[
            ("broken.template.yml", "test_template: '{{ broken'\n"),
            ("test.spec.yml", "test:\n  route: https://example.com\n"),
        ]);

        let report = run(&project);
        let diagnostics = diagnostics_with_code(&report, REPORT_INVALID_LIQUID);

        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .context
                .as_deref()
                .unwrap()
                .contains("broken")
        );
    }

    #[test]
    fn lowercase_supported_verb_is_valid() {
        let (_directory, project) = discover_files(&[(
            "valid.spec.yml",
            "test:\n  route: https://example.com\n  verb: get\n",
        )]);

        assert!(diagnostics_with_code(&run(&project), TEST_UNSUPPORTED_VERB).is_empty());
    }

    #[test]
    fn whitespace_route_and_empty_describe_are_reported() {
        let (_directory, project) = discover_files(&[
            ("blank-route.spec.yml", "test:\n  route: '   '\n"),
            ("empty.spec.yml", "describe: []\n"),
        ]);

        let report = run(&project);

        assert_eq!(diagnostics_with_code(&report, TEST_EMPTY_ROUTE).len(), 1);
        assert_eq!(diagnostics_with_code(&report, DESCRIPTOR_EMPTY).len(), 1);
    }
}
