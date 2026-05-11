use std::time::Duration;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct RunResult{
    pub success: bool,
    pub http_result: Option<HttpResult>,
    pub assertions: Option<Vec<Assertion>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpResult {
    #[serde(skip)]
    pub status: StatusCode,
    #[serde(skip)]
    pub headers: HeaderMap,
    pub body: String,
    #[serde(skip)]
    pub json: Option<serde_json::Value>,
    #[serde(skip)]
    pub bytes: Vec<u8>,
    pub duration: Duration,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Assertion{
    pub expr: String,
    pub passed: bool,
    pub error: String,
}