use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportTemplateModel {
    pub test_template: Option<String>,
    pub section_template: Option<String>,
    pub error_template: Option<String>,
    pub title_template: Option<String>,
    pub summary_template: Option<String>,
    pub file: Option<ReportFileModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportFileModel {
    pub dir: Option<PathBuf>,
    pub file_name: Option<String>,
}
