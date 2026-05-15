mod liquid;
mod sinks;

use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use crate::models::report_template::ReportTemplate;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::reporting::liquid::LiquidReporter;
use enum_dispatch::enum_dispatch;
use std::collections::HashMap;

#[enum_dispatch]
pub trait Reporter {
    fn report(
        &self,
        descriptor: &Descriptor,
        test_result: Option<&TestResult>,
        assertions: &[Assertion],
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
    );
    fn summary(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        results: &[SummaryResult],
    );
    fn title(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
    );
}

#[enum_dispatch(Reporter)]
pub enum AnyReporter {
    LiquidReporter,
}

pub fn reporter_for() -> AnyReporter {
    LiquidReporter.into()
}
