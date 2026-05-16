use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use crate::models::run_options::RunOptions;
use crate::models::report_template::{ReportFile};
use crate::pipeline::reporting::sinks::OutputSink;
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::templating::TemplateEngine;

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
        let name = match liquid.render(name, &obj){
            Ok(output) => &output.clone(),
            Err(_) => name,
        };

        Self { path: dir.join(name) }
    }

    fn open_append(&self) -> std::fs::File {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).expect("Failed to create report directory");
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("Failed to open report file for appending")
    }
}

impl OutputSink for FileSink {
    fn println(&self, msg: &str) {
        let mut file = self.open_append();
        _ = writeln!(file, "{}", msg).map_err(|e|println!("{e}"));
    }

    fn print(&self, msg: &str) {
        let mut file = self.open_append();
        _ = write!(file, "{}", msg).map_err(|e|println!("{e}"));
    }
}