use crate::pipeline::reporting::sinks::OutputSink;
use std::io::Write;

pub struct ConsoleSink;
impl OutputSink for ConsoleSink {
    fn println(&self, msg: &str) -> anyhow::Result<()> {
        writeln!(std::io::stdout().lock(), "{msg}")?;
        Ok(())
    }

    fn print(&self, msg: &str) -> anyhow::Result<()> {
        write!(std::io::stdout().lock(), "{msg}")?;
        Ok(())
    }
}
