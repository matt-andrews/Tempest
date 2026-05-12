use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct OptionsModel {
    #[serde(rename = "base_uri")]
    pub base_uri: Option<String>,
    pub debug: Option<bool>,
    pub reports: Option<Vec<String>>,
}

impl OptionsModel {
    pub fn default() -> Self{
        Self{
            base_uri: None,
            debug: Some(false),
            reports: Some(Vec::new()),
        }
    }
    pub fn merge(self, other: OptionsModel) -> OptionsModel {
        OptionsModel {
            base_uri: other.base_uri.or(self.base_uri),
            debug: other.debug.or(self.debug),
            reports: other.reports.or(self.reports),
        }
    }
}