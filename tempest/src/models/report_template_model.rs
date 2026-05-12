use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportTemplateModel{
    pub test: Option<String>,
    pub section: Option<String>,
    pub error: Option<String>,
    pub file: Option<ReportFileModel>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportFileModel{
    pub dir: Option<PathBuf>,
    pub file_name: Option<String>,
}