mod console;
mod file;

use crate::models::report_template::ReportTemplate;
use crate::models::run_options::RunOptions;
use crate::pipeline::reporting::sinks::console::ConsoleSink;
use crate::pipeline::reporting::sinks::file::FileSink;
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait OutputSink {
    fn println(&self, msg: &str) -> anyhow::Result<()>;
    fn print(&self, msg: &str) -> anyhow::Result<()>;
}

#[enum_dispatch(OutputSink)]
pub enum AnyOutputSink {
    ConsoleSink,
    FileSink,
}

pub fn output_sink_for(template: &ReportTemplate, options: &RunOptions) -> AnyOutputSink {
    if let Some(file_cfg) = &template.file {
        FileSink::new(file_cfg, options).into()
    } else {
        ConsoleSink.into()
    }
}
