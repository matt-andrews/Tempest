use crate::utils::header_map_converter::header_map_serde;
use liquid::model::Value;
use reqwest::StatusCode;
use reqwest::header::{HeaderMap, ToStrError};
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

impl TestResult {
    pub fn to_liquid_template(&self) -> liquid::Object {
        let headers = match headers_to_liquid(&self.headers) {
            Ok(val) => val,
            Err(_) => liquid::Object::new(),
        };
        liquid::object!({
            "status": self.status.code as i64,
            "status_message": self.status.message.clone(),
            "body": self.body.clone(),
            "duration_ms": self.duration.as_secs_f64() * 1000.0,
            "json": self.json.clone(),
            "headers": headers,
        })
    }
}

pub fn headers_to_liquid(headers: &HeaderMap) -> anyhow::Result<liquid::Object> {
    let mut object = liquid::Object::new();

    for name in headers.keys() {
        let mut values = headers
            .get_all(name)
            .iter()
            .map(|value| value.to_str().map(|text| Value::scalar(text.to_owned())))
            .collect::<Result<Vec<Value>, ToStrError>>()?;

        let liquid_value = if values.len() == 1 {
            values.pop().expect("HeaderMap key must have a value")
        } else {
            Value::array(values)
        };

        object.insert(name.as_str().to_owned().into(), liquid_value);
    }

    Ok(object)
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
