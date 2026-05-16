pub mod assertions;
mod reporting;
pub mod runners;
pub mod templating;
pub mod variables;

use crate::discovery::DiscoveryResult;
use crate::models::run_context::RunContext;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assertions::{AssertionEvaluator, assertion_evaluator_for};
use crate::pipeline::reporting::{reporter_for, Reporter};
use crate::pipeline::runners::{TestRunner, test_runner_for};
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::templating::TemplateEngine;
use crate::pipeline::variables::{variable_assignment_for, VariableAssignment};

pub async fn execute(
    discovery_result: &DiscoveryResult,
    default_options: &RunOptions,
) -> anyhow::Result<()> {
    let report_provider = reporter_for();
    let discovered = &discovery_result.directory;

    let template_engine = LiquidEngine;

    let top_level_options = default_options.clone().merge(
        discovered
            .options
            .iter()
            .cloned()
            .reduce(|acc, next| acc.merge(next))
            .unwrap_or_default(),
    );

    report_provider.title(
        &top_level_options,
        &discovery_result.templates,
        discovered.test_count(),
    );

    let mut summary: Vec<SummaryResult> = Vec::new();

    for directory in discovered.walk() {
        let base_options = default_options.clone().merge(
            directory
                .options
                .iter()
                .cloned()
                .reduce(|acc, next| acc.merge(next))
                .unwrap_or_default(),
        );

        let mut context = RunContext::new("", &directory.envs);

        for (descriptor, ancestor_options) in directory.files.iter().flat_map(|m| m.descendants()) {
            let mut descriptor = descriptor.to_owned();

            if let Some(file_name) = descriptor.file.clone() {
                let file_name = template_engine.render_string_or_self(
                    &file_name,
                    &liquid::object!(&context),
                );

                if file_name != context.file_name {
                    context = RunContext::new(&file_name, &directory.envs);
                }

                descriptor.file = Some(file_name);
            }

            let mut options = base_options
                .clone()
                .merge(ancestor_options)
                .merge(descriptor.options.clone().unwrap_or_default());

            options.render_template(&template_engine, &liquid::object!(&context));
            descriptor.render_template(&template_engine, &liquid::object!(&context));

            let mut assert_result: Vec<Assertion> = Vec::new();
            let mut test_result: Option<TestResult> = None;

            if let Some(test) = &descriptor.test {
                let mut test = test.to_owned();
                test.render_template(&template_engine, &liquid::object!(&context));

                let test_runner = test_runner_for(&test, &options);
                test_result = Some(test_runner.run().await);
                for assert in &test.assert.unwrap_or_default() {
                    let assert_evaluator = assertion_evaluator_for(assert);
                    let result = assert_evaluator.evaluate(test_result.as_ref().unwrap());
                    assert_result.push(result.clone());
                }

                for var in &test.vars.unwrap_or_default() {
                    let assign_var = variable_assignment_for(var);
                    assign_var.set(test_result.as_ref().unwrap(), &mut context);
                }

                summary.push(match assert_result.iter().any(|a| !a.passed) {
                    true => SummaryResult::Failed,
                    false => SummaryResult::Passed,
                });
            }

            report_provider.report(
                &descriptor,
                test_result.as_ref(),
                &assert_result,
                &options,
                &discovery_result.templates,
                summary.len()
            );
        }
    }

    report_provider.summary(&top_level_options, &discovery_result.templates, &summary);

    Ok(())
}