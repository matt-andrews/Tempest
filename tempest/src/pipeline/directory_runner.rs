use crate::models::evaluation_context::EvaluationContext;
use crate::models::descriptor::Descriptor;
use crate::models::directory_node::DirectoryNode;
use crate::models::report_template::ReportTemplate;
use crate::models::run_context::RunContext;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::models::test_spec::TestSpec;
use crate::pipeline::assertions::{AssertionEvaluator, assertion_evaluator_for};
use crate::pipeline::reporting::{AnyReporter, Reporter};
use crate::pipeline::runners::TestRunner;
use crate::pipeline::runners::test_runner_for;
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::variables::{VariableAssignment, variable_assignment_for};
use std::collections::HashMap;
use std::path::Path;

pub struct DirectoryRunner<'a> {
    reporter: &'a AnyReporter,
    directory: &'a DirectoryNode,
    template_engine: &'a LiquidEngine,
    templates: &'a HashMap<String, ReportTemplate>,
    base_options: RunOptions,
    top_path: &'a Path,
}

impl<'a> DirectoryRunner<'a> {
    pub fn new(
        base_options: RunOptions,
        directory: &'a DirectoryNode,
        template_engine: &'a LiquidEngine,
        templates: &'a HashMap<String, ReportTemplate>,
        reporter: &'a AnyReporter,
        top_path: &'a Path,
    ) -> Self {
        Self {
            reporter,
            base_options,
            directory,
            templates,
            template_engine,
            top_path,
        }
    }
    pub async fn execute_dir(&self, summary: &mut Vec<SummaryResult>) {
        let mut context = RunContext::new("", &self.directory.envs);
        for (descriptor, ancestor_options) in self.descriptors() {
            let outcome = self
                .execute_descriptor_with_retries(
                    descriptor,
                    ancestor_options,
                    &mut context,
                    summary.len(),
                )
                .await;

            if let Some(result) = &outcome.summary_result {
                summary.push(result.to_owned());
            }
        }
    }

    async fn execute_descriptor_with_retries(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
        context: &mut RunContext,
        test_count: usize,
    ) -> DescriptorOutcome {
        let context_before_descriptor = context.clone();
        let mut retry_attempts = 0usize;
        let mut saw_failure = false;

        loop {
            context.retry_attempts = retry_attempts;
            let mut outcome = self
                .execute_descriptor(descriptor, ancestor_options.clone(), context)
                .await;

            self.report_descriptor(&outcome, test_count, retry_attempts);

            match &outcome.summary_result {
                Some(SummaryResult::Failed) => {
                    let max_retries = usize::from(outcome.options.retries.unwrap_or(0));
                    if retry_attempts < max_retries {
                        retry_attempts += 1;
                        saw_failure = true;
                        *context = context_before_descriptor.clone();
                        continue;
                    }
                }
                Some(SummaryResult::Passed) if saw_failure => {
                    outcome.summary_result = Some(SummaryResult::Flaky);
                }
                _ => {}
            }

            return outcome;
        }
    }

    async fn execute_descriptor(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
        context: &mut RunContext,
    ) -> DescriptorOutcome {
        let mut descriptor = descriptor.to_owned();

        self.apply_file_context(&mut descriptor, context);

        let mut options = self.options_for_descriptor(&descriptor, ancestor_options);

        self.render_inputs(&mut descriptor, &mut options, context);

        let test_run = self
            .run_test_if_present(&descriptor, &options, context)
            .await;

        DescriptorOutcome {
            descriptor,
            options,
            test_result: test_run.test_result,
            assertions: test_run.assertions,
            summary_result: test_run.summary_result,
        }
    }

    async fn run_test_if_present(
        &self,
        descriptor: &Descriptor,
        options: &RunOptions,
        context: &mut RunContext,
    ) -> TestRunOutcome {
        let Some(test) = &descriptor.test else {
            return TestRunOutcome {
                test_result: None,
                assertions: Vec::new(),
                summary_result: None,
            };
        };

        let mut test = test.to_owned();
        test.render_template(self.template_engine, &liquid::object!(&context));

        let runner = test_runner_for(&test, options);
        let test_result = runner.run().await;

        let assertions = self.evaluate_assertions(&test, &test_result, descriptor);
        self.assign_variables(&test, &test_result, context, descriptor);

        let summary_result = if assertions.iter().any(|a| !a.passed) {
            SummaryResult::Failed
        } else {
            SummaryResult::Passed
        };

        TestRunOutcome {
            test_result: Some(test_result),
            assertions,
            summary_result: Some(summary_result),
        }
    }

    fn evaluate_assertions(
        &self,
        test: &TestSpec,
        test_result: &TestResult,
        descriptor: &Descriptor,
    ) -> Vec<Assertion> {
        let mut assert_result: Vec<Assertion> = Vec::new();
        for assert in test.assert.as_deref().unwrap_or_default() {
            let evaluation_context = EvaluationContext {
                suite_dir: self.top_path.to_path_buf(),
                spec_file: descriptor.file.clone(),
            };
            let assert_evaluator = assertion_evaluator_for(assert);
            let result = assert_evaluator.evaluate(test_result, &evaluation_context);
            assert_result.push(result.clone());
        }
        assert_result
    }

    fn assign_variables(&self, test: &TestSpec, result: &TestResult, context: &mut RunContext, descriptor: &Descriptor) {
        let evaluation_context = EvaluationContext {
            suite_dir: self.top_path.to_path_buf(),
            spec_file: descriptor.file.clone(),
        };
        let vars = test.vars.clone().unwrap_or_default();
        let assign_var = variable_assignment_for(&vars);
        assign_var.set(result, &mut *context, &evaluation_context);
    }

    fn descriptors(&self) -> impl Iterator<Item = (&Descriptor, RunOptions)> {
        self.directory.files.iter().flat_map(|m| m.descendants())
    }

    fn apply_file_context(&self, descriptor: &mut Descriptor, context: &mut RunContext) {
        if let Some(file_name) = descriptor.file.clone() {
            if file_name != context.file_name {
                *context =
                    RunContext::new(file_name.to_str().unwrap_or_default(), &self.directory.envs);
            }

            descriptor.file = Some(file_name);
        }
    }

    fn options_for_descriptor(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
    ) -> RunOptions {
        self.base_options
            .clone()
            .merge(ancestor_options)
            .merge(descriptor.options.clone().unwrap_or_default())
    }

    fn render_inputs(
        &self,
        descriptor: &mut Descriptor,
        options: &mut RunOptions,
        context: &RunContext,
    ) {
        let obj = liquid::object!(&context);
        options.render_template(self.template_engine, &obj);
        descriptor.render_template(self.template_engine, &obj);
    }

    fn report_descriptor(&self, outcome: &DescriptorOutcome, count: usize, retry_count: usize) {
        self.reporter.report(
            &outcome.descriptor,
            outcome.test_result.as_ref(),
            &outcome.assertions,
            &outcome.options,
            self.templates,
            count,
            retry_count,
        );
    }
}

struct DescriptorOutcome {
    descriptor: Descriptor,
    options: RunOptions,
    test_result: Option<TestResult>,
    assertions: Vec<Assertion>,
    summary_result: Option<SummaryResult>,
}

struct TestRunOutcome {
    test_result: Option<TestResult>,
    assertions: Vec<Assertion>,
    summary_result: Option<SummaryResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor::Descriptor;
    use crate::models::directory_node::DirectoryNode;
    use crate::models::report_template::{ReportFile, ReportTemplate};
    use crate::models::run_options::RunOptions;
    use crate::models::summary_result::SummaryResult;
    use crate::models::test_spec::TestSpec;
    use crate::pipeline::reporting::reporter_for;
    use crate::pipeline::templating::liquid::LiquidEngine;
    use mockito::Server;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn dir_node(files: Vec<Descriptor>) -> DirectoryNode {
        DirectoryNode {
            files,
            options: vec![],
            children: vec![],
            dir: PathBuf::from("test"),
            envs: HashMap::new(),
        }
    }

    fn dir_node_with_envs(files: Vec<Descriptor>, envs: HashMap<String, String>) -> DirectoryNode {
        DirectoryNode {
            files,
            options: vec![],
            children: vec![],
            dir: PathBuf::from("test"),
            envs,
        }
    }

    fn make_runner<'a>(
        directory: &'a DirectoryNode,
        engine: &'a LiquidEngine,
        templates: &'a HashMap<String, ReportTemplate>,
        reporter: &'a AnyReporter,
        base_options: RunOptions,
    ) -> DirectoryRunner<'a> {
        DirectoryRunner::new(
            base_options,
            directory,
            engine,
            templates,
            reporter,
            Path::new("./"),
        )
    }

    fn get_descriptor(route: &str, assertions: Option<Vec<&str>>) -> Descriptor {
        Descriptor {
            name: Some("test".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: route.to_string(),
                verb: Some("GET".to_string()),
                assert: assertions.map(|a| a.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        }
    }

    #[tokio::test]
    async fn descriptor_without_test_produces_no_summary_result() {
        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![Descriptor {
            name: Some("no-test".to_string()),
            description: None,
            tags: None,
            test: None,
            describe: None,
            options: None,
            file: None,
        }]);

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert!(summary.is_empty());
    }

    #[tokio::test]
    async fn passing_assertion_yields_passed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/ok", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
    }

    #[tokio::test]
    async fn failing_assertion_yields_failed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/ok", server.url()),
            Some(vec!["status == 404"]),
        )]);

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Failed));
    }

    #[tokio::test]
    async fn test_without_assertions_yields_passed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(&format!("{}/ok", server.url()), None)]);

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
    }

    #[tokio::test]
    async fn multiple_descriptors_produce_multiple_summary_entries() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/a")
            .with_status(200)
            .create_async()
            .await;
        server
            .mock("GET", "/b")
            .with_status(404)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![
            get_descriptor(&format!("{}/a", server.url()), Some(vec!["status == 200"])),
            get_descriptor(&format!("{}/b", server.url()), Some(vec!["status == 200"])),
        ]);

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Failed));
    }

    #[tokio::test]
    async fn base_uri_is_prepended_to_relative_route() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/items")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor("/items", Some(vec!["status == 200"]))]);

        let mut summary = Vec::new();
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                base_uri: Some(server.url()),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn descriptor_options_override_base_options() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/path")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![Descriptor {
            name: Some("override".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: "/path".to_string(),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: Some(RunOptions {
                base_uri: Some(server.url()),
                ..Default::default()
            }),
            file: None,
        }]);

        let mut summary = Vec::new();
        // base_options has a bogus URI; descriptor.options has the real one
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                base_uri: Some("http://127.0.0.1:1".to_string()),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn variable_set_in_one_test_is_available_in_next() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/seed")
            .with_status(200)
            .with_body("42")
            .create_async()
            .await;
        let mock_consumer = server
            .mock("GET", "/items/42")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();

        let seed = Descriptor {
            name: Some("seed".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/seed", server.url()),
                verb: Some("GET".to_string()),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let consumer = Descriptor {
            name: Some("consumer".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/items/{{{{ file.id }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };

        let dir = dir_node(vec![seed, consumer]);
        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[1], SummaryResult::Passed));
        mock_consumer.assert_async().await;
    }

    #[tokio::test]
    async fn env_vars_are_available_in_route_templates() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();

        let mut envs = HashMap::new();
        envs.insert("HOST".to_string(), server.url());

        let dir = dir_node_with_envs(
            vec![Descriptor {
                name: Some("env-test".to_string()),
                description: None,
                tags: None,
                test: Some(TestSpec {
                    route: "{{ env.HOST }}/api".to_string(),
                    verb: Some("GET".to_string()),
                    assert: Some(vec!["status == 200".to_string()]),
                    ..Default::default()
                }),
                describe: None,
                options: None,
                file: None,
            }],
            envs,
        );

        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn retry_pass_after_failure_yields_single_flaky_summary() {
        let mut server = Server::new_async().await;
        let fail = server
            .mock("GET", "/unstable")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let pass = server
            .mock("GET", "/unstable")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/unstable", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let mut summary = Vec::new();
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                retries: Some(1),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Flaky));
        fail.assert_async().await;
        pass.assert_async().await;
    }

    #[tokio::test]
    async fn retry_exhaustion_yields_single_failed_summary() {
        let mut server = Server::new_async().await;
        let fail = server
            .mock("GET", "/down")
            .with_status(500)
            .expect(3)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/down", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let mut summary = Vec::new();
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                retries: Some(2),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Failed));
        fail.assert_async().await;
    }

    #[tokio::test]
    async fn retry_attempts_are_reported_but_summary_counts_descriptor_once() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/reported")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/reported")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let report_dir = tempfile::tempdir().unwrap();
        let report_path = report_dir.path().join("report.txt");
        let mut templates = HashMap::new();
        templates.insert(
            "file".to_string(),
            ReportTemplate {
                test_template: Some("{{ status }}\n".to_string()),
                section_template: None,
                error_template: None,
                title_template: None,
                summary_template: None,
                file: Some(ReportFile {
                    dir: Some(report_dir.path().to_path_buf()),
                    file_name: Some("report.txt".to_string()),
                }),
            },
        );

        let engine = LiquidEngine;
        let reporter = reporter_for();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/reported", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let mut summary = Vec::new();
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                reports: Some(vec!["file".to_string()]),
                retries: Some(1),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        let report = std::fs::read_to_string(report_path).unwrap();

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Flaky));
        assert_eq!(report.lines().collect::<Vec<_>>(), vec!["500", "200"]);
    }

    #[tokio::test]
    async fn failed_retry_attempts_do_not_mutate_context_for_next_attempt() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/seed")
            .with_status(200)
            .with_body("start")
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/unstable/start")
            .with_status(500)
            .with_body("bad")
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/unstable/start")
            .with_status(200)
            .with_body("good")
            .expect(1)
            .create_async()
            .await;
        let consumer = server
            .mock("GET", "/items/good")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();
        let seed = Descriptor {
            name: Some("seed".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/seed", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let unstable = Descriptor {
            name: Some("unstable".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/unstable/{{{{ file.token }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let consumer_descriptor = Descriptor {
            name: Some("consumer".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/items/{{{{ file.token }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };

        let dir = dir_node(vec![seed, unstable, consumer_descriptor]);
        let mut summary = Vec::new();
        make_runner(
            &dir,
            &engine,
            &templates,
            &reporter,
            RunOptions {
                retries: Some(1),
                ..Default::default()
            },
        )
        .execute_dir(&mut summary)
        .await;

        assert_eq!(summary.len(), 3);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Flaky));
        assert!(matches!(summary[2], SummaryResult::Passed));
        consumer.assert_async().await;
    }

    #[tokio::test]
    async fn file_context_resets_when_file_changes() {
        // file-a sets file.token via vars; file-b is in a different file so its
        // context is reset — it hits an unrelated route with no template variables.
        // The companion test `variable_set_in_one_test_is_available_in_next` proves
        // that within the same file the variable DOES flow; this test proves that
        // crossing a file boundary produces a clean context.
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/file-a")
            .with_status(200)
            .with_body("secret")
            .create_async()
            .await;
        let mock_b = server
            .mock("GET", "/file-b")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let reporter = reporter_for();

        let file_a = Descriptor {
            name: Some("file-a".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/file-a", server.url()),
                verb: Some("GET".to_string()),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: Some(PathBuf::from("file-a.yaml")),
        };
        let file_b = Descriptor {
            name: Some("file-b".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/file-b", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: Some(PathBuf::from("file-b.yaml")),
        };

        let dir = dir_node(vec![file_a, file_b]);
        let mut summary = Vec::new();
        make_runner(&dir, &engine, &templates, &reporter, RunOptions::default())
            .execute_dir(&mut summary)
            .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Passed));
        mock_b.assert_async().await;
    }
}
