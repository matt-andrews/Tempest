use crate::models::descriptor::Descriptor;
use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::run_context::RunContext;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::models::test_spec::TestSpec;
use crate::pipeline::assertions::{AssertionEvaluator, assertion_evaluator_for};
use crate::pipeline::runners::{TestRunner, resolved_route, test_runner_for};
use crate::pipeline::variables::{VariableAssignment, variable_assignment_for};
use crate::templating::{TemplateEngine, liquid::LiquidEngine};
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct FileRunner<'a> {
    ordinal: usize,
    root: &'a Descriptor,
    envs: &'a HashMap<String, String>,
    base_options: RunOptions,
    template_engine: &'a LiquidEngine,
    suite_path: &'a Path,
}

impl<'a> FileRunner<'a> {
    pub fn new(
        ordinal: usize,
        root: &'a Descriptor,
        envs: &'a HashMap<String, String>,
        base_options: RunOptions,
        template_engine: &'a LiquidEngine,
        suite_path: &'a Path,
    ) -> Self {
        Self {
            ordinal,
            root,
            envs,
            base_options,
            template_engine,
            suite_path,
        }
    }
    pub async fn execute(&self) -> FileOutcome {
        let file_name = self
            .root
            .file
            .as_deref()
            .and_then(Path::to_str)
            .unwrap_or_default();

        let mut context = RunContext::new(file_name, self.envs);

        let mut descriptors: Vec<DescriptorOutcome> = Vec::new();
        for (descriptor, ancestor_options, ancestor_titles) in self.root.descendants() {
            let outcome = self
                .execute_descriptor_with_retries(
                    descriptor,
                    ancestor_options,
                    &ancestor_titles,
                    &mut context,
                )
                .await;

            descriptors.push(outcome);
        }
        FileOutcome {
            ordinal: self.ordinal,
            descriptors,
            source_file: self.root.file.clone().unwrap_or_default(),
        }
    }

    async fn execute_descriptor_with_retries(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
        ancestor_titles: &[String],
        context: &mut RunContext,
    ) -> DescriptorOutcome {
        let original_context = context.clone();
        let mut attempts = Vec::new();
        let mut retry_number = 0;
        let mut saw_failure = false;

        loop {
            context.retry_attempts = retry_number;

            let attempt = self
                .execute_attempt(
                    descriptor,
                    ancestor_options.clone(),
                    ancestor_titles,
                    context,
                )
                .await;

            let attempt_result = attempt.result.clone();
            let max_retries = usize::from(attempt.options.retries.unwrap_or(0));

            attempts.push(attempt);

            let final_result = match attempt_result {
                Some(SummaryResult::Failed) if retry_number < max_retries => {
                    retry_number += 1;
                    saw_failure = true;
                    *context = original_context.clone();
                    continue;
                }

                Some(SummaryResult::Passed) if saw_failure => Some(SummaryResult::Flaky),

                result => result,
            };

            return DescriptorOutcome {
                attempts,
                final_result,
            };
        }
    }

    async fn execute_attempt(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
        ancestor_titles: &[String],
        context: &mut RunContext,
    ) -> AttemptOutcome {
        let mut descriptor = descriptor.to_owned();

        let mut options = self.options_for_descriptor(&descriptor, ancestor_options);

        self.render_inputs(&mut descriptor, &mut options, context);
        let title_path = self.render_title_path(ancestor_titles, context);

        let test_run = self
            .run_test_if_present(&descriptor, &options, context)
            .await;

        AttemptOutcome {
            descriptor,
            title_path,
            options,
            test_result: test_run.test_result,
            assertions: test_run.assertions,
            result: test_run.summary_result,
            debug_message: test_run.debug_message,
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
                debug_message: None,
            };
        };

        if let Some(skip) = &options.skip
            && skip.resolve(self.template_engine, &liquid::object!(&context))
                .unwrap_or(false)
        {
            return TestRunOutcome {
                test_result: None,
                assertions: Vec::new(),
                summary_result: Some(SummaryResult::Skipped),
                debug_message: None,
            };
        }

        let mut test = test.to_owned();
        test.render_template(self.template_engine, &liquid::object!(&context));

        let runner = test_runner_for(&test, options);
        let test_result = runner.run().await;

        let debug_message = match options.debug.unwrap_or(false) {
            true => Some(self.format_debug_message(&test, options, &test_result)),
            false => None,
        };

        let response_content_cache = ResponseContentCache::default();
        let assertions =
            self.evaluate_assertions(&test, &test_result, descriptor, &response_content_cache);
        self.assign_variables(
            &test,
            &test_result,
            context,
            descriptor,
            &response_content_cache,
        );

        let summary_result = if assertions.iter().any(|a| !a.passed) {
            SummaryResult::Failed
        } else {
            SummaryResult::Passed
        };

        TestRunOutcome {
            test_result: Some(test_result),
            assertions,
            summary_result: Some(summary_result),
            debug_message,
        }
    }

    fn format_debug_message(
        &self,
        test_spec: &TestSpec,
        options: &RunOptions,
        test_result: &TestResult,
    ) -> String {
        format!(
            "Route: {} - {}\nHeaders: \n{}\nBody: \n{}",
            resolved_route(test_spec, options),
            test_result.status.code,
            self.headers_to_display(&test_result.headers),
            test_result.body
        )
    }

    fn headers_to_display(&self, headers: &HeaderMap) -> String {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        for (key, value) in headers.iter() {
            let k = key.as_str().to_string();
            let v = String::from_utf8_lossy(value.as_bytes()).to_string();
            map.entry(k).or_default().push(v);
        }

        let result: Vec<String> = map
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v.join(";")))
            .collect();
        result.join("\n")
    }

    fn evaluate_assertions(
        &self,
        test: &TestSpec,
        test_result: &TestResult,
        descriptor: &Descriptor,
        response_content_cache: &ResponseContentCache,
    ) -> Vec<Assertion> {
        let mut assert_result: Vec<Assertion> = Vec::new();
        for assert in test.assert.as_deref().unwrap_or_default() {
            let evaluation_context = EvaluationContext {
                suite_dir: self.suite_path.to_path_buf(),
                spec_file: descriptor.file.clone(),
            };
            let assert_evaluator = assertion_evaluator_for(assert);
            let result =
                assert_evaluator.evaluate(test_result, &evaluation_context, response_content_cache);
            assert_result.push(result.clone());
        }
        assert_result
    }

    fn assign_variables(
        &self,
        test: &TestSpec,
        result: &TestResult,
        context: &mut RunContext,
        descriptor: &Descriptor,
        response_content_cache: &ResponseContentCache,
    ) {
        let evaluation_context = EvaluationContext {
            suite_dir: self.suite_path.to_path_buf(),
            spec_file: descriptor.file.clone(),
        };
        let vars = test.vars.clone().unwrap_or_default();
        let assign_var = variable_assignment_for(&vars);
        assign_var.set(
            result,
            &mut *context,
            &evaluation_context,
            response_content_cache,
        );
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

    fn render_title_path(&self, titles: &[String], context: &RunContext) -> Vec<String> {
        let obj = liquid::object!(&context);
        titles
            .iter()
            .map(|title| self.template_engine.render_string_or_self(title, &obj))
            .collect()
    }
}

pub struct AttemptOutcome {
    pub descriptor: Descriptor,
    pub title_path: Vec<String>,
    pub options: RunOptions,
    pub test_result: Option<TestResult>,
    pub assertions: Vec<Assertion>,
    pub result: Option<SummaryResult>,
    pub debug_message: Option<String>,
}

pub struct DescriptorOutcome {
    pub attempts: Vec<AttemptOutcome>,
    pub final_result: Option<SummaryResult>,
}

pub struct FileOutcome {
    pub ordinal: usize,
    pub source_file: PathBuf,
    pub descriptors: Vec<DescriptorOutcome>,
}

struct TestRunOutcome {
    test_result: Option<TestResult>,
    assertions: Vec<Assertion>,
    summary_result: Option<SummaryResult>,
    debug_message: Option<String>,
}
