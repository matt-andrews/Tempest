use std::collections::HashMap;
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
use crate::pipeline::runners::{test_runner_for};
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::templating::TemplateEngine;
use crate::pipeline::variables::{variable_assignment_for, VariableAssignment};
use crate::pipeline::runners::TestRunner;

pub struct DirectoryRunner<'a> {
    reporter: &'a AnyReporter,
    directory: &'a DirectoryNode,
    template_engine: &'a LiquidEngine,
    templates: &'a HashMap<String, ReportTemplate>,
    base_options: RunOptions,
}

impl<'a> DirectoryRunner<'a> {
    pub fn new(
        base_options: RunOptions,
        directory: &'a DirectoryNode,
        template_engine: &'a LiquidEngine,
        templates: &'a HashMap<String, ReportTemplate>,
        reporter: &'a AnyReporter,
    ) -> Self{
        Self{
            reporter,
            base_options,
            directory,
            templates,
            template_engine,
        }
    }
    pub async fn execute_dir(&self, summary: &mut Vec<SummaryResult>){
        let mut context = RunContext::new("", &self.directory.envs);
        for (descriptor, ancestor_options) in self.descriptors() {
            let outcome = self
                .execute_descriptor(descriptor, ancestor_options, &mut context)
                .await;

            if let Some(result) = &outcome.summary_result {
                summary.push(result.to_owned());
            }

            self.report_descriptor(&outcome, summary.len());
        }
    }

    async fn execute_descriptor(
        &self,
        descriptor: &Descriptor,
        ancestor_options: RunOptions,
        context: &mut RunContext
    ) -> DescriptorOutcome {
        let mut descriptor = descriptor.to_owned();

        self.apply_file_context(&mut descriptor, context);

        let mut options = self.options_for_descriptor(&descriptor, ancestor_options);

        self.render_inputs(&mut descriptor, &mut options, context);

        let test_run = self.run_test_if_present(&descriptor, &options, context).await;

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
        context: &mut RunContext
    ) -> TestRunOutcome {
        let Some(test) = &descriptor.test else {
            return TestRunOutcome {
                test_result: None,
                assertions: Vec::new(),
                summary_result: None,
            };
        };

        let mut test = test.to_owned();
        test.render_template(&self.template_engine, &liquid::object!(&context));

        let runner = test_runner_for(&test, options);
        let test_result = runner.run().await;

        let assertions = self.evaluate_assertions(&test, &test_result);
        self.assign_variables(&test, &test_result, context);

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

    fn evaluate_assertions(&self, test: &TestSpec, test_result: &TestResult) -> Vec<Assertion> {
        let mut assert_result: Vec<Assertion> = Vec::new();
        for assert in &test.assert.clone().unwrap_or_default() {
            let assert_evaluator = assertion_evaluator_for(assert);
            let result = assert_evaluator.evaluate(test_result);
            assert_result.push(result.clone());
        }
        assert_result
    }

    fn assign_variables(&self, test: &TestSpec, result: &TestResult, context: &mut RunContext) {
        for var in &test.vars.clone().unwrap_or_default() {
            let assign_var = variable_assignment_for(var);
            assign_var.set(result, &mut *context);
        }
    }

    fn descriptors(&self) -> impl Iterator<Item = (&Descriptor, RunOptions)> {
        self.directory.files.iter().flat_map(|m| m.descendants())
    }

    fn apply_file_context(&self,
                          descriptor: &mut Descriptor,
                          context: &mut RunContext) {
        if let Some(file_name) = descriptor.file.clone() {
            let file_name = self.template_engine.render_string_or_self(
                &file_name,
                &liquid::object!(&context),
            );

            if file_name != context.file_name {
                *context = RunContext::new(&file_name, &self.directory.envs);
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
        context: &RunContext
    ) {
        let obj = liquid::object!(&context);
        options.render_template(&self.template_engine, &obj);
        descriptor.render_template(&self.template_engine, &obj);
    }

    fn report_descriptor(&self, outcome: &DescriptorOutcome, count: usize){
        self.reporter.report(
            &outcome.descriptor,
            outcome.test_result.as_ref(),
            &outcome.assertions,
            &outcome.options,
            &self.templates,
            count
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