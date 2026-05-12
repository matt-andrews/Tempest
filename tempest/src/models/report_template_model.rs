use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportTemplateModel{
    pub test: Option<String>,
    pub section: Option<String>,
    pub error: Option<String>,
}