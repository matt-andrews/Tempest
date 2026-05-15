mod cli_output;
mod file_output;

use enum_dispatch::enum_dispatch;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::pipeline::report_capabilities::output_capabilities::cli_output::CliOutput;
use crate::pipeline::report_capabilities::output_capabilities::file_output::FileOutput;

#[enum_dispatch]
pub trait OutputCapability{
    fn println(&self, msg: &str);
    fn print(&self, msg: &str);
}

#[enum_dispatch(OutputCapability)]
pub enum OutputCapabilityProvider{
    CliOutput,
    FileOutput,
}

pub fn get_output(template: &ReportTemplateModel, options: &OptionsModel) -> OutputCapabilityProvider{
    if let Some(file_cfg) = &template.file {
        FileOutput::new(file_cfg, options).into()
    } else {
        CliOutput.into()
    }
}