use crate::utils::header_map_converter::header_map_serde;
use std::time::Duration;
use reqwest::header::{HeaderMap};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TestResult{
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
pub struct Assertion{
    pub expr: String,
    pub passed: bool,
    pub error: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TempestStatusCode{
    pub code: u16,
    pub message: String,
}

impl TempestStatusCode{
    pub fn from_status(status: StatusCode) -> TempestStatusCode{
        TempestStatusCode{
            code: status.as_u16(),
            message: status.to_string(),
        }
    }
    pub fn from_message(msg: String) -> TempestStatusCode{
        TempestStatusCode{
            code: 504,
            message: msg.to_string(),
        }
    }

    pub fn to_display(&self) -> String{
        format!("{} {}", self.code, self.message)
    }
}