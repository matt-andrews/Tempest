mod cli_output;
mod file_output;

use enum_dispatch::enum_dispatch;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::pipeline::report_capabilities::output_capabilities::cli_output::CliOutput;

#[enum_dispatch]
pub trait OutputCapability{
    fn println(&self, msg: &str);
    fn print(&self, msg: &str);
}

#[enum_dispatch(OutputCapability)]
pub enum OutputCapabilityProvider{
    CliOutput
}

pub fn get_output(template: &ReportTemplateModel, options: &OptionsModel) -> OutputCapabilityProvider{
    CliOutput.into()
}