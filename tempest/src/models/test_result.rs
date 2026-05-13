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
            message: status.canonical_reason().unwrap_or_default().to_string(),
        }
    }
    pub fn from_message(msg: String) -> TempestStatusCode{
        TempestStatusCode{
            code: 504,
            message: msg.to_string(),
        }
    }

    pub fn to_display(&self) -> String{
        format!("{}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_status_captures_code_and_canonical_reason() {
        let s = TempestStatusCode::from_status(StatusCode::OK);
        assert_eq!(s.code, 200);
        assert_eq!(s.message, "OK");
    }

    #[test]
    fn from_status_404_not_found() {
        let s = TempestStatusCode::from_status(StatusCode::NOT_FOUND);
        assert_eq!(s.code, 404);
        assert_eq!(s.message, "Not Found");
    }

    #[test]
    fn from_message_always_uses_504() {
        let s = TempestStatusCode::from_message("connection refused".to_string());
        assert_eq!(s.code, 504);
        assert_eq!(s.message, "connection refused");
    }

    #[test]
    fn to_display_returns_message_string() {
        let s = TempestStatusCode { code: 200, message: "OK".to_string() };
        assert_eq!(s.to_display(), "OK");
    }
}