mod liquid_reporter;
mod output_capabilities;

use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::report_capabilities::liquid_reporter::LiquidReporter;
use enum_dispatch::enum_dispatch;
use std::collections::HashMap;

#[enum_dispatch]
pub trait ReportCapability {
    fn report(
        &self,
        descriptor: &DescriptorModel,
        test_result: Option<&TestResult>,
        assertions: &[Assertion],
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
    );
    fn summary(
        &self,
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
        results: &[SummaryResult],
    );
    fn title(
        &self,
        options: &OptionsModel,
        templates: &HashMap<String, ReportTemplateModel>,
        test_count: usize,
    );
}

#[enum_dispatch(ReportCapability)]
pub enum ReportCapabilityProvider {
    LiquidReporter,
}

pub fn get_report_capability() -> ReportCapabilityProvider {
    LiquidReporter.into()
}
