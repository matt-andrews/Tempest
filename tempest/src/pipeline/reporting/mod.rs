pub mod event;
mod liquid;
mod sinks;
pub mod template_reporter;

use crate::models::descriptor::Descriptor;
use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::reporting::template_reporter::TemplateReporter;
use enum_dispatch::enum_dispatch;
use std::collections::HashMap;

#[enum_dispatch]
pub trait Reporter {
    //i need to figure out how to fix this later.
    #[allow(clippy::too_many_arguments)]
    fn report(
        &self,
        descriptor: &Descriptor,
        title_path: &[String],
        expansion_prefix: &str,
        test_result: Option<&TestResult>,
        assertions: &[Assertion],
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
        retry_count: usize,
    ) -> anyhow::Result<()>;
    fn debug(
        &self,
        msg: &str,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
    ) -> anyhow::Result<()>;
    fn summary(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        results: &[SummaryResult],
    ) -> anyhow::Result<SummaryResult>;
    fn title(
        &self,
        options: &RunOptions,
        templates: &HashMap<String, ReportTemplate>,
        test_count: usize,
    ) -> anyhow::Result<()>;
}

#[enum_dispatch(Reporter)]
pub enum AnyReporter {
    TemplateReporter,
}

pub fn reporter_for() -> AnyReporter {
    TemplateReporter::new().into()
}
