use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::file_runner::FileOutcome;
use crate::pipeline::reporting::{AnyReporter, Reporter, reporter_for};
use std::collections::{BTreeMap, HashMap};

pub struct ReportCoordinator<'a> {
    reporter: AnyReporter,
    templates: &'a HashMap<String, ReportTemplate>,
    results: Vec<SummaryResult>,
    pending: BTreeMap<usize, FileOutcome>,
    next_file_ordinal: usize,
}

impl<'a> ReportCoordinator<'a> {
    pub fn new(templates: &'a HashMap<String, ReportTemplate>) -> Self {
        Self {
            reporter: reporter_for(),
            templates,
            results: Vec::new(),
            pending: BTreeMap::new(),
            next_file_ordinal: 0,
        }
    }

    pub fn title(&self, options: &RunOptions, test_count: usize) -> anyhow::Result<()> {
        self.reporter.title(options, self.templates, test_count)
    }

    pub fn consume(&mut self, outcome: FileOutcome) -> anyhow::Result<()> {
        let ordinal = outcome.ordinal;
        let source_file = outcome.source_file.clone();

        if let Some(existing) = self.pending.insert(ordinal, outcome) {
            panic!(
                "duplicate file outcome ordinal {ordinal}: {} and {}",
                existing.source_file.display(),
                source_file.display()
            );
        }

        while let Some(outcome) = self.pending.remove(&self.next_file_ordinal) {
            self.emit_file(outcome)?;
            self.next_file_ordinal += 1;
        }
        Ok(())
    }

    pub fn summary(&self, options: &RunOptions) -> anyhow::Result<SummaryResult> {
        assert!(
            self.pending.is_empty(),
            "cannot summarize while file outcomes are missing"
        );
        self.reporter
            .summary(options, self.templates, &self.results)
    }

    #[cfg(test)]
    pub fn results(&self) -> &[SummaryResult] {
        &self.results
    }

    fn emit_file(&mut self, outcome: FileOutcome) -> anyhow::Result<()> {
        for descriptor in outcome.descriptors {
            let test_count = self.results.len();

            for (retry_count, attempt) in descriptor.attempts.iter().enumerate() {
                if let Some(message) = attempt.debug_message.as_deref() {
                    self.reporter
                        .debug(message, &attempt.options, self.templates)?;
                }

                self.reporter.report(
                    &attempt.descriptor,
                    &attempt.title_path,
                    attempt.test_result.as_ref(),
                    &attempt.assertions,
                    &attempt.options,
                    self.templates,
                    test_count,
                    retry_count,
                )?;
            }

            if let Some(result) = descriptor.final_result {
                self.results.push(result);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery;
    use crate::models::descriptor::Descriptor;
    use crate::models::report_template::{ReportFile, ReportTemplate};
    use crate::models::test_result::TestResult;
    use crate::models::test_spec::TestSpec;
    use crate::pipeline::file_runner::{AttemptOutcome, DescriptorOutcome};
    use std::path::{Path, PathBuf};

    fn template(dir: &Path) -> ReportTemplate {
        ReportTemplate {
            title_template: None,
            section_template: Some("section:{{ name }}:{{ test_count }}\n".to_string()),
            test_template: Some(
                "attempt:{{ name }}:{{ test_count }}:{{ retry_count }}\n".to_string(),
            ),
            summary_template: None,
            error_template: None,
            debug_template: Some("debug:{{ debug_message }}\n".to_string()),
            file: Some(ReportFile {
                dir: Some(dir.to_path_buf()),
                file_name: Some("report.txt".to_string()),
            }),
        }
    }

    fn options(debug: bool) -> RunOptions {
        RunOptions {
            reports: Some(vec!["test".to_string()]),
            debug: Some(debug),
            ..Default::default()
        }
    }

    fn attempt(
        name: &str,
        has_test: bool,
        result: Option<SummaryResult>,
        retry_debug: Option<&str>,
    ) -> AttemptOutcome {
        AttemptOutcome {
            descriptor: Descriptor {
                name: Some(name.to_string()),
                description: None,
                tags: None,
                test: has_test.then(TestSpec::default),
                describe: None,
                options: None,
                file: Some(PathBuf::from(format!("{name}.spec.yml"))),
            },
            title_path: Vec::new(),
            options: options(retry_debug.is_some()),
            test_result: has_test.then(TestResult::default),
            assertions: Vec::new(),
            result,
            debug_message: retry_debug.map(str::to_string),
        }
    }

    fn descriptor(
        attempts: Vec<AttemptOutcome>,
        final_result: Option<SummaryResult>,
    ) -> DescriptorOutcome {
        DescriptorOutcome {
            attempts,
            final_result,
        }
    }

    fn file(ordinal: usize, descriptors: Vec<DescriptorOutcome>) -> FileOutcome {
        FileOutcome {
            ordinal,
            source_file: PathBuf::from(format!("{ordinal}.spec.yml")),
            descriptors,
        }
    }

    #[test]
    fn out_of_order_file_outcomes_are_emitted_in_ordinal_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.txt");
        let templates = HashMap::from([("test".to_string(), template(dir.path()))]);
        let mut coordinator = ReportCoordinator::new(&templates);

        coordinator
            .consume(file(
                1,
                vec![descriptor(
                    vec![attempt("second", true, Some(SummaryResult::Failed), None)],
                    Some(SummaryResult::Failed),
                )],
            ))
            .unwrap();

        assert!(!path.exists(), "later files must wait for earlier ordinals");

        coordinator
            .consume(file(
                0,
                vec![descriptor(
                    vec![attempt("first", true, Some(SummaryResult::Passed), None)],
                    Some(SummaryResult::Passed),
                )],
            ))
            .unwrap();

        let output = std::fs::read_to_string(path).unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            vec!["attempt:first:0:0", "attempt:second:1:0"]
        );
        assert_eq!(
            coordinator.results(),
            &[SummaryResult::Passed, SummaryResult::Failed]
        );
    }

    #[test]
    fn retries_stay_adjacent_and_count_as_one_logical_test() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.txt");
        let templates = HashMap::from([("test".to_string(), template(dir.path()))]);
        let mut coordinator = ReportCoordinator::new(&templates);

        coordinator
            .consume(file(
                0,
                vec![
                    descriptor(
                        vec![
                            attempt(
                                "unstable",
                                true,
                                Some(SummaryResult::Failed),
                                Some("/unstable"),
                            ),
                            attempt(
                                "unstable",
                                true,
                                Some(SummaryResult::Passed),
                                Some("/unstable"),
                            ),
                        ],
                        Some(SummaryResult::Flaky),
                    ),
                    descriptor(
                        vec![attempt("next", true, Some(SummaryResult::Passed), None)],
                        Some(SummaryResult::Passed),
                    ),
                ],
            ))
            .unwrap();

        let output = std::fs::read_to_string(path).unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            vec![
                "debug:/unstable",
                "attempt:unstable:0:0",
                "debug:/unstable",
                "attempt:unstable:0:1",
                "attempt:next:1:0",
            ]
        );
        assert_eq!(
            coordinator.results(),
            &[SummaryResult::Flaky, SummaryResult::Passed]
        );
    }

    #[test]
    fn sections_do_not_increment_the_logical_test_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.txt");
        let templates = HashMap::from([("test".to_string(), template(dir.path()))]);
        let mut coordinator = ReportCoordinator::new(&templates);

        coordinator
            .consume(file(
                0,
                vec![
                    descriptor(vec![attempt("group", false, None, None)], None),
                    descriptor(
                        vec![attempt("test", true, Some(SummaryResult::Passed), None)],
                        Some(SummaryResult::Passed),
                    ),
                ],
            ))
            .unwrap();

        let output = std::fs::read_to_string(path).unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            vec!["section:group:0", "attempt:test:0:0"]
        );
    }

    #[test]
    fn built_in_json_report_is_valid_when_files_complete_out_of_order() {
        let suite_dir = tempfile::tempdir().unwrap();
        let report_dir = tempfile::tempdir().unwrap();
        let report_path = report_dir.path().join("report.json");
        let mut discovered = discovery::discover(
            suite_dir.path(),
            None,
            &mut HashMap::new(),
            suite_dir.path(),
        )
        .unwrap();
        let mut json_template = discovered.templates.remove("json").unwrap();
        json_template.file = Some(ReportFile {
            dir: Some(report_dir.path().to_path_buf()),
            file_name: Some("report.json".to_string()),
        });
        let templates = HashMap::from([("json".to_string(), json_template)]);
        let options = RunOptions {
            reports: Some(vec!["json".to_string()]),
            ..Default::default()
        };
        let mut coordinator = ReportCoordinator::new(&templates);

        coordinator.title(&options, 2).unwrap();
        coordinator
            .consume(file(
                1,
                vec![descriptor(
                    vec![AttemptOutcome {
                        options: options.clone(),
                        ..attempt("second", true, Some(SummaryResult::Passed), None)
                    }],
                    Some(SummaryResult::Passed),
                )],
            ))
            .unwrap();
        coordinator
            .consume(file(
                0,
                vec![descriptor(
                    vec![AttemptOutcome {
                        options: options.clone(),
                        ..attempt("first", true, Some(SummaryResult::Passed), None)
                    }],
                    Some(SummaryResult::Passed),
                )],
            ))
            .unwrap();

        assert_eq!(
            coordinator.summary(&options).unwrap(),
            SummaryResult::Passed
        );

        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(report_path).unwrap()).unwrap();
        assert_eq!(report["tests"][0]["name"], "first");
        assert_eq!(report["tests"][1]["name"], "second");
        assert_eq!(report["summary"]["passed"], 2);
    }
}
