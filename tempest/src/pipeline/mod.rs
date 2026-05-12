pub mod test_capabilities;
pub mod assert_capabilities;
mod report_capabilities;

use crate::pipeline::test_capabilities::{get_test_capability, TestCapability};
use crate::models::directory_model::DirectoryModel;
use crate::models::options_model::OptionsModel;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assert_capabilities::{get_assert_capability, AssertCapability};
use crate::pipeline::report_capabilities::get_report_capability;
use crate::pipeline::report_capabilities::ReportCapability;

pub async fn execute(discovered: DirectoryModel, default_options: OptionsModel) -> anyhow::Result<()>{
    for directory in discovered.walk() {
        let base_options = default_options
            .clone()
            .merge(directory.options.iter()
                .cloned()
                .reduce(|acc, next| acc.merge(next))
                .unwrap_or_default()
            );

        for (descriptor, ancestor_options) in directory.files.iter().flat_map(|m| m.descendants()) {

            let options = base_options.clone()
                .merge(ancestor_options)
                .merge(descriptor.options.clone().unwrap_or_default());

            let mut assert_result: Vec<Assertion> = Vec::new();
            let mut test_result: Option<TestResult> = None;

            if let Some(test) = &descriptor.test{
                let test_capability = get_test_capability(descriptor, &test, &options);
                test_result = Some(test_capability.test().await);
                for assert in test.assert.clone().unwrap_or_default(){
                    let assert_capability = get_assert_capability(assert);
                    let result = assert_capability.assert(&test_result.as_ref().unwrap());
                    assert_result.push(result.clone());
                }
            }

            let report_provider = get_report_capability();
            report_provider.report(&descriptor, test_result, assert_result, options);
        }
    }

    Ok(())
}