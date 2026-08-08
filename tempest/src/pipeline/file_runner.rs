use crate::cel::{self, LetBindings};
use crate::models::descriptor::{Descriptor, Profile};
use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::run_context::{IterationContext, RunContext};
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::models::test_spec::TestSpec;
use crate::pipeline::assertions::{AssertionEvaluator, assertion_evaluator_for};
use crate::pipeline::runners::{TestRunner, resolved_route, test_runner_for};
use crate::pipeline::variables::{VariableAssignment, variable_assignment_for};
use crate::templating::{TemplateEngine, liquid::LiquidEngine};
use reqwest::header::HeaderMap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

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
        let mut title_path = Vec::new();
        self.execute_descriptor_tree(
            self.root,
            RunOptions::default(),
            &mut context,
            &mut title_path,
            &mut descriptors,
        )
        .await;
        FileOutcome {
            ordinal: self.ordinal,
            descriptors,
            source_file: self.root.file.clone().unwrap_or_default(),
        }
    }

    fn execute_descriptor_tree<'b>(
        &'b self,
        descriptor: &'b Descriptor,
        ancestor_options: RunOptions,
        context: &'b mut RunContext,
        title_path: &'b mut Vec<String>,
        outcomes: &'b mut Vec<DescriptorOutcome>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'b>> {
        Box::pin(async move {
            let profiles = descriptor
                .profiles
                .as_ref()
                .map(|profiles| profiles.iter().cloned().map(Some).collect::<Vec<_>>())
                .unwrap_or_else(|| vec![None]);
            let profile_count = profiles.len();
            let loop_count = descriptor
                .options
                .as_ref()
                .and_then(|options| options.loop_count)
                .map_or(1, |count| count.get());
            let has_profile = descriptor.profiles.is_some();
            let has_loop = descriptor
                .options
                .as_ref()
                .is_some_and(|options| options.loop_count.is_some());
            let expanded = has_profile || has_loop;
            let case_count = profile_count * loop_count;
            let mut child_options = ancestor_options
                .clone()
                .merge(descriptor.options.clone().unwrap_or_default());
            child_options.loop_count = None;

            for (profile_index, profile) in profiles.into_iter().enumerate() {
                for loop_index in 0..loop_count {
                    let scope = if expanded {
                        let profile = profile
                            .as_ref()
                            .map(|profile| self.render_profile(profile, context));
                        Some(context.enter_iteration(
                            profile,
                            IterationContext {
                                case_index: profile_index * loop_count + loop_index,
                                case_count,
                                has_profile,
                                profile_index,
                                profile_count,
                                has_loop,
                                loop_index,
                                loop_count,
                            },
                        ))
                    } else {
                        None
                    };

                    let outcome = self
                        .execute_descriptor_with_retries(
                            descriptor,
                            ancestor_options.clone(),
                            title_path,
                            context,
                        )
                        .await;
                    let rendered_name = outcome
                        .attempts
                        .last()
                        .and_then(|attempt| attempt.descriptor.name.as_deref())
                        .filter(|name| !name.trim().is_empty())
                        .map(str::to_owned);
                    outcomes.push(outcome);

                    if let Some(name) = &rendered_name {
                        title_path.push(name.clone());
                    }

                    for child in descriptor.describe.as_deref().unwrap_or_default() {
                        self.execute_descriptor_tree(
                            child,
                            child_options.clone(),
                            context,
                            title_path,
                            outcomes,
                        )
                        .await;
                    }

                    if rendered_name.is_some() {
                        title_path.pop();
                    }
                    if let Some(scope) = scope {
                        context.exit_iteration(scope);
                    }
                }
            }
        })
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
            let retry_delay = attempt.options.retry_delay();

            attempts.push(attempt);

            let final_result = match attempt_result {
                Some(SummaryResult::Failed) if retry_number < max_retries => {
                    retry_number += 1;
                    saw_failure = true;
                    *context = original_context.clone();
                    if !retry_delay.is_zero() {
                        tokio::time::sleep(retry_delay).await;
                    }

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
        let title_path = ancestor_titles.to_vec();
        let expansion_prefix = context.expansion_prefix();

        let test_run = self
            .run_test_if_present(&descriptor, &options, context)
            .await;

        AttemptOutcome {
            descriptor,
            title_path,
            expansion_prefix,
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
            && skip
                .resolve(self.template_engine, &liquid::object!(&context))
                .unwrap_or_else(|e| {
                    eprintln!("{e}");
                    false
                })
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
        let let_bindings = match self.evaluate_let_bindings(
            &test,
            &test_result,
            descriptor,
            &response_content_cache,
        ) {
            Ok(bindings) => bindings,
            Err(assertion) => {
                return TestRunOutcome {
                    test_result: Some(test_result),
                    assertions: vec![assertion],
                    summary_result: Some(SummaryResult::Failed),
                    debug_message,
                };
            }
        };
        let assertions = self.evaluate_assertions(
            &test,
            &test_result,
            descriptor,
            &response_content_cache,
            &let_bindings,
        );
        self.assign_variables(
            &test,
            &test_result,
            context,
            descriptor,
            &response_content_cache,
            &let_bindings,
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

    fn evaluate_let_bindings(
        &self,
        test: &TestSpec,
        test_result: &TestResult,
        descriptor: &Descriptor,
        response_content_cache: &ResponseContentCache,
    ) -> Result<LetBindings, Assertion> {
        let mut bindings = LetBindings::new();
        let Some(declarations) = &test.lets else {
            return Ok(bindings);
        };
        let evaluation_context = EvaluationContext {
            suite_dir: self.suite_path.to_path_buf(),
            spec_file: descriptor.file.clone(),
        };

        for (name, expression) in declarations {
            let value = cel::evaluate(
                expression,
                test_result,
                &evaluation_context,
                response_content_cache,
                &bindings,
            )
            .map_err(|error| Assertion {
                expr: format!("let.{name} = {expression}"),
                passed: false,
                error: error.to_string(),
            })?;
            bindings.insert(name.clone(), value);
        }

        Ok(bindings)
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
        let_bindings: &LetBindings,
    ) -> Vec<Assertion> {
        let mut assert_result: Vec<Assertion> = Vec::new();
        for assert in test.assert.as_deref().unwrap_or_default() {
            let evaluation_context = EvaluationContext {
                suite_dir: self.suite_path.to_path_buf(),
                spec_file: descriptor.file.clone(),
            };
            let assert_evaluator = assertion_evaluator_for(assert);
            let result = assert_evaluator.evaluate(
                test_result,
                &evaluation_context,
                response_content_cache,
                let_bindings,
            );
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
        let_bindings: &LetBindings,
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
            let_bindings,
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

    fn render_profile(&self, profile: &Profile, context: &RunContext) -> Profile {
        profile
            .iter()
            .map(|(key, value)| {
                (
                    self.template_engine
                        .render_string_or_self(key, &liquid::object!(&context)),
                    self.render_profile_value(value, context),
                )
            })
            .collect()
    }

    fn render_profile_value(&self, value: &JsonValue, context: &RunContext) -> JsonValue {
        match value {
            JsonValue::String(value) => JsonValue::String(
                self.template_engine
                    .render_string_or_self(value, &liquid::object!(&context)),
            ),
            JsonValue::Array(values) => JsonValue::Array(
                values
                    .iter()
                    .map(|value| self.render_profile_value(value, context))
                    .collect(),
            ),
            JsonValue::Object(values) => JsonValue::Object(
                values
                    .iter()
                    .map(|(key, value)| {
                        (
                            self.template_engine
                                .render_string_or_self(key, &liquid::object!(&context)),
                            self.render_profile_value(value, context),
                        )
                    })
                    .collect(),
            ),
            value => value.clone(),
        }
    }
}

pub struct AttemptOutcome {
    pub descriptor: Descriptor,
    pub title_path: Vec<String>,
    pub expansion_prefix: String,
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
