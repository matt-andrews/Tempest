use crate::models::descriptor::Descriptor;
use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::reporting::Reporter;
use crate::pipeline::reporting::event::ReportEvent;
use crate::pipeline::reporting::liquid::LiquidRenderer;
use crate::pipeline::reporting::sinks::OutputSink;
use crate::pipeline::reporting::sinks::output_sink_for;
use std::collections::HashMap;

pub struct TemplateReporter {
    renderer: LiquidRenderer,
}

impl TemplateReporter {
    pub fn new() -> Self {
        Self {
            renderer: LiquidRenderer::new(),
        }
    }
    fn emit(
        &self,
        event: ReportEvent<'_>,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
    ) {
        for template in active_templates(templates, options) {
            let sink = output_sink_for(template, options);
            match self.renderer.render(template, &event) {
                Ok(output) => sink.print(&output),
                Err(err) => {
                    match self.renderer.render(
                        template,
                        &ReportEvent::Error {
                            msg: &err.to_string(),
                        },
                    ) {
                        Ok(fallback) => sink.print(&fallback),
                        Err(err) => {
                            sink.println(&format!("Failed to render and fallback: {}", err))
                        }
                    };
                }
            }
        }
    }
}

impl Reporter for TemplateReporter {
    fn debug(&self, msg: &str, options: &RunOptions, templates: &HashMap<String, ReportTemplate>) {
        self.emit(ReportEvent::Debug { msg }, options, templates);
    }

    fn report(
        &self,
        descriptor: &Descriptor,
        test_result: Option<&TestResult>,
        assertions: &[Assertion],
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
        retry_count: usize,
    ) {
        self.emit(
            ReportEvent::Descriptor {
                descriptor,
                test_result,
                assertions,
                test_count,
                retry_count,
            },
            options,
            templates,
        );
    }

    fn summary(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        results: &[SummaryResult],
    ) -> SummaryResult {
        let passed = results
            .iter()
            .filter(|f| matches!(f, SummaryResult::Passed))
            .count();
        let failed = results
            .iter()
            .filter(|f| matches!(f, SummaryResult::Failed))
            .count();
        let flaky = results
            .iter()
            .filter(|f| matches!(f, SummaryResult::Flaky))
            .count();
        self.emit(
            ReportEvent::Summary {
                passed,
                failed,
                flaky,
            },
            options,
            templates,
        );

        if failed > 0 {
            SummaryResult::Failed
        } else if flaky > 0 {
            SummaryResult::Flaky
        } else {
            SummaryResult::Passed
        }
    }

    fn title(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
    ) {
        self.emit(ReportEvent::Title { test_count }, options, templates);
    }
}

fn active_templates<'a>(
    templates: &'a HashMap<String, ReportTemplate>,
    options: &RunOptions,
) -> Vec<&'a ReportTemplate> {
    let report_names = options.reports.as_deref().unwrap_or_default();
    report_names
        .iter()
        .filter_map(|name| templates.get(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::report_template::{ReportFile, ReportTemplate};
    use crate::models::run_options::RunOptions;
    use std::collections::HashMap;

    fn report_template(name: &str) -> ReportTemplate {
        ReportTemplate {
            title_template: Some(name.to_string()),
            test_template: Some(name.to_string()),
            section_template: Some(name.to_string()),
            summary_template: Some(name.to_string()),
            error_template: Some(name.to_string()),
            debug_template: Some(name.to_string()),
            file: None,
        }
    }

    fn options_reports(reports: &[&str]) -> RunOptions {
        RunOptions {
            base_uri: None,
            debug: None,
            reports: Some(reports.iter().map(|r| r.to_string()).collect()),
            start_time: None,
            retries: Some(0),
        }
    }

    #[test]
    fn active_templates_returns_only_templates_named_in_options() {
        let mut templates = HashMap::new();
        templates.insert("console".to_string(), report_template("console"));
        templates.insert("json".to_string(), report_template("json"));
        templates.insert("html".to_string(), report_template("html"));

        let active = active_templates(&templates, &options_reports(&["console", "html"]));
        let rendered_names: Vec<_> = active
            .iter()
            .map(|t| t.title_template.as_deref().unwrap())
            .collect();

        assert_eq!(active.len(), 2);
        assert!(rendered_names.contains(&"console"));
        assert!(rendered_names.contains(&"html"));
        assert!(!rendered_names.contains(&"json"));
    }

    #[test]
    fn active_templates_is_empty_when_reports_are_not_configured() {
        let mut templates = HashMap::new();
        templates.insert("console".to_string(), report_template("console"));

        let options = RunOptions {
            base_uri: None,
            debug: None,
            reports: None,
            start_time: None,
            retries: Some(0),
        };

        assert!(active_templates(&templates, &options).is_empty());
    }

    #[test]
    fn render_error_fallback_is_written_to_file_sink() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.txt");

        let template = ReportTemplate {
            title_template: Some("{{ broken".to_string()),
            test_template: None,
            section_template: None,
            summary_template: None,
            error_template: Some("ERR: {{ liquid_error_message }}".to_string()),
            debug_template: None,
            file: Some(ReportFile {
                dir: Some(dir.path().to_path_buf()),
                file_name: Some("report.txt".to_string()),
            }),
        };

        let mut templates = HashMap::new();
        templates.insert("file".to_string(), template);

        let options = RunOptions {
            base_uri: None,
            debug: None,
            reports: Some(vec!["file".to_string()]),
            start_time: None,
            retries: Some(0),
        };

        TemplateReporter::new().title(&options, &templates, 1);

        let output = std::fs::read_to_string(path).unwrap();
        assert!(output.contains("ERR:"));
        assert!(output.contains("liquid"));
    }
}
