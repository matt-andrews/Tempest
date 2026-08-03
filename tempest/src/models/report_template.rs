use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportTemplate {
    pub test_template: Option<String>,
    pub section_template: Option<String>,
    pub error_template: Option<String>,
    pub debug_template: Option<String>,
    pub title_template: Option<String>,
    pub summary_template: Option<String>,
    pub file: Option<ReportFile>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportFile {
    pub dir: Option<PathBuf>,
    pub file_name: Option<String>,
}
