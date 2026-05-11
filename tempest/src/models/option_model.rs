use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OptionModel{
    #[serde(rename="baseUri")]
    pub base_uri: String,
    pub debug: bool,
}