pub mod assert_capabilities;
mod report_capabilities;
pub mod test_capabilities;

use crate::discovery::DiscoveryResult;
use crate::models::options_model::OptionsModel;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assert_capabilities::{AssertCapability, get_assert_capability};
use crate::pipeline::report_capabilities::ReportCapability;
use crate::pipeline::report_capabilities::get_report_capability;
use crate::pipeline::test_capabilities::{TestCapability, get_test_capability};

pub async fn execute(
    discovery_result: &DiscoveryResult,
    default_options: &OptionsModel,
) -> anyhow::Result<()> {
    let report_provider = get_report_capability();
    let discovered = &discovery_result.directory;

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

        for (descriptor, ancestor_options) in directory.files.iter().flat_map(|m| m.descendants()) {
            let options = base_options
                .clone()
                .merge(ancestor_options)
                .merge(descriptor.options.clone().unwrap_or_default());

            let mut assert_result: Vec<Assertion> = Vec::new();
            let mut test_result: Option<TestResult> = None;

            if let Some(test) = &descriptor.test {
                let test_capability = get_test_capability(&test, &options);
                test_result = Some(test_capability.test().await);
                for assert in test.assert.clone().unwrap_or_default() {
                    let assert_capability = get_assert_capability(&assert);
                    let result = assert_capability.assert(&test_result.as_ref().unwrap());
                    assert_result.push(result.clone());
                }
            }
            summary.push(match assert_result.iter().any(|a| !a.passed) {
                true => SummaryResult::Failed,
                false => SummaryResult::Passed,
            });

            report_provider.report(
                &descriptor,
                test_result.as_ref(),
                &assert_result,
                &options,
                &discovery_result.templates,
            );
        }
    }

    report_provider.summary(&top_level_options, &discovery_result.templates, &summary);

    Ok(())
}
