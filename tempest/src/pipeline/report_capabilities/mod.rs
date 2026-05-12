mod console_reporter;
mod liquid_reporter;

use std::collections::HashMap;
use enum_dispatch::enum_dispatch;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::liquid_reporter::LiquidReporter;

#[enum_dispatch]
pub trait ReportCapability{
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<TestResult>,
        assertions: Vec<Assertion>,
        options: OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>
    );
}

#[enum_dispatch(ReportCapability)]
pub enum ReportCapabilityProvider{
    LiquidReporter
}

pub fn get_report_capability() -> ReportCapabilityProvider{
    LiquidReporter.into()
}