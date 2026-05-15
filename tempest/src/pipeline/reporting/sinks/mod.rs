mod console;
mod file;

use enum_dispatch::enum_dispatch;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use crate::pipeline::reporting::sinks::console::ConsoleSink;
use crate::pipeline::reporting::sinks::file::FileSink;

#[enum_dispatch]
pub trait OutputSink {
    fn println(&self, msg: &str);
    fn print(&self, msg: &str);
}

#[enum_dispatch(OutputSink)]
pub enum AnyOutputSink {
    ConsoleSink,
    FileSink,
}

pub fn output_sink_for(template: &ReportTemplateModel, options: &OptionsModel) -> AnyOutputSink {
    if let Some(file_cfg) = &template.file {
        FileSink::new(file_cfg, options).into()
    } else {
        ConsoleSink.into()
    }
}