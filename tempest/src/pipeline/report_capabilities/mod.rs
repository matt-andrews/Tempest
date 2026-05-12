mod console_reporter;

use enum_dispatch::enum_dispatch;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::console_reporter::ConsoleReporter;

#[enum_dispatch]
pub trait ReportCapability{
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<TestResult>,
        assertions: Vec<Assertion>,
        options: OptionsModel,
    );
}

#[enum_dispatch(ReportCapability)]
pub enum ReportCapabilityProvider{
    ConsoleReporter
}

pub fn get_report_capability() -> ReportCapabilityProvider{
    ConsoleReporter.into()
}