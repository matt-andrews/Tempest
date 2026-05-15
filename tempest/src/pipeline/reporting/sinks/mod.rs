mod cli_output;
mod file_output;

use enum_dispatch::enum_dispatch;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::pipeline::reporting::output_capabilities::cli_output::ConsoleSink;
use crate::pipeline::reporting::output_capabilities::file_output::FileSink;

#[enum_dispatch]
pub trait OutputSink {
    fn println(&self, msg: &str);
    fn print(&self, msg: &str);
}

#[enum_dispatch(OutputSink)]
pub enum OutputSinkProvider {
    ConsoleSink,
    FileSink,
}

pub fn get_output_sink(template: &ReportTemplateModel, options: &OptionsModel) -> OutputSinkProvider {
    if let Some(file_cfg) = &template.file {
        FileSink::new(file_cfg, options).into()
    } else {
        ConsoleSink.into()
    }
}