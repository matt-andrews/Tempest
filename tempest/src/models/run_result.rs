/*use std::time::Duration;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct RunResult{
    pub http_result: HttpResult,
    pub assertions: Vec<Assertion>,
    pub stop: bool,
}

#[derive(Clone, Debug, Default)]
pub struct HttpResult {
    pub status: TempestStatusCode,
    pub headers: HeaderMap,
    pub body: String,
    pub json: Option<serde_json::Value>,
    pub bytes: Vec<u8>,
    pub duration: Duration,
}

#[derive(Clone, Debug, Default)]
pub struct Assertion{
    pub expr: String,
    pub passed: bool,
    pub error: String,
}*/