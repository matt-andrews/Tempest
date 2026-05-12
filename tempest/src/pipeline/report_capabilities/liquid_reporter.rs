use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::ReportCapability;

pub struct LiquidReporter;

impl ReportCapability for LiquidReporter {
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<TestResult>,
        assertions: Vec<Assertion>,
        options: OptionsModel
    ) {
        
    }
}