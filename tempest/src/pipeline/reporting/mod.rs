mod liquid;
mod sinks;
pub mod event;
pub mod template_reporter;

use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use crate::models::report_template::ReportTemplate;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use enum_dispatch::enum_dispatch;
use std::collections::HashMap;
use crate::pipeline::reporting::template_reporter::TemplateReporter;

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
    TemplateReporter,
}

pub fn reporter_for() -> AnyReporter {
    TemplateReporter::new().into()
}
