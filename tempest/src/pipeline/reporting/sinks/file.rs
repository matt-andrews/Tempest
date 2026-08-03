use crate::models::report_template::ReportFile;
use crate::models::run_options::RunOptions;
use crate::pipeline::reporting::sinks::OutputSink;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use anyhow::Context;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub struct FileSink {
    path: PathBuf,
}

impl FileSink {
    pub fn new(file_cfg: &ReportFile, options: &RunOptions) -> Self {
        let dir = file_cfg.dir.clone().unwrap_or_else(|| PathBuf::from("."));
        let name = file_cfg.file_name.as_deref().unwrap_or("report.txt");
        let obj = liquid::object!({
            "start_timestamp": options.start_time.unwrap_or_default().unix_timestamp()
        });

        let liquid = LiquidEngine;
        let name = match liquid.render(name, &obj) {
            Ok(output) => &output.clone(),
            Err(_) => name,
        };

        Self {
            path: dir.join(name),
        }
    }

    fn open_append(&self) -> anyhow::Result<std::fs::File> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("failed to create report directory {}", dir.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "failed to open report file {} for appending",
                    self.path.display()
                )
            })?;
        Ok(file)
    }
}

impl OutputSink for FileSink {
    fn println(&self, msg: &str) -> anyhow::Result<()> {
        let mut file = self.open_append()?;
        writeln!(file, "{msg}")
            .with_context(|| format!("failed to write report file {}", self.path.display()))?;
        Ok(())
    }

    fn print(&self, msg: &str) -> anyhow::Result<()> {
        let mut file = self.open_append()?;
        write!(file, "{msg}")
            .with_context(|| format!("failed to write report file {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_directory_failure_is_returned_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-directory");
        std::fs::write(&blocker, "file").unwrap();

        let sink = FileSink::new(
            &ReportFile {
                dir: Some(blocker.join("reports")),
                file_name: Some("report.txt".to_string()),
            },
            &RunOptions::default(),
        );

        let error = sink.print("result").unwrap_err();
        assert!(format!("{error:#}").contains("failed to create report directory"));
    }
}
