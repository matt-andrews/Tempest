use crate::utils::header_map_converter::header_map_serde;
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TestResult {
    pub status: TempestStatusCode,
    #[serde(with = "header_map_serde")]
    pub headers: HeaderMap,
    pub body: String,
    #[serde(skip)]
    pub json: Option<serde_json::Value>,
    #[serde(skip)]
    pub bytes: Vec<u8>,
    pub duration: Duration,
}

#[derive(Clone, Debug, Default)]
pub struct Assertion {
    pub expr: String,
    pub passed: bool,
    pub error: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TempestStatusCode {
    pub code: u16,
    pub message: String,
}

impl TempestStatusCode {
    pub fn from_status(status: StatusCode) -> TempestStatusCode {
        TempestStatusCode {
            code: status.as_u16(),
            message: status.canonical_reason().unwrap_or_default().to_string(),
        }
    }
    pub fn from_message(msg: String) -> TempestStatusCode {
        TempestStatusCode {
            code: 504,
            message: msg,
        }
    }
}

impl std::fmt::Display for TempestStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
