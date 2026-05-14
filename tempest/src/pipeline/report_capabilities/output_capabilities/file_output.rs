use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use crate::models::report_template_model::{ReportFileModel};
use crate::pipeline::report_capabilities::output_capabilities::OutputCapability;

pub struct FileOutput {
    path: PathBuf,
}

impl FileOutput {
    pub fn new(file_cfg: &ReportFileModel) -> Self {
        let dir = file_cfg.dir.clone().unwrap_or_else(|| PathBuf::from("."));
        let name = file_cfg.file_name.as_deref().unwrap_or("report.txt");
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

impl OutputCapability for FileOutput {
    fn println(&self, msg: &str) {
        let mut file = self.open_append();
        _ = writeln!(file, "{}", msg).map_err(|e|println!("{e}"));
    }

    fn print(&self, msg: &str) {
        let mut file = self.open_append();
        _ = write!(file, "{}", msg).map_err(|e|println!("{e}"));
    }
}