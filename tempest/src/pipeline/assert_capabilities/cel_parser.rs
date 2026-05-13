use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assert_capabilities::AssertCapability;
use cel_interpreter::objects::Key;
use cel_interpreter::{Context, Program, Value};
use reqwest::header::HeaderMap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CelParser {
    assertion: String,
}
impl AssertCapability for CelParser {
    fn assert(&self, data: &TestResult) -> Assertion {
        let result = Self::evaluate_assertion(&self.assertion, data).map_err(|e| e.to_string());
        let error = match &result {
            Ok(_) => String::new(),
            Err(e) => e.clone(),
        };
        Assertion {
            expr: self.assertion.clone(),
            error,
            passed: result.unwrap_or(false),
        }
    }
}
impl CelParser {
    pub fn new(assertion: &str) -> Self {
        Self {
            assertion: assertion.to_string(),
        }
    }
    fn json_to_cel(val: &JsonValue) -> Value {
        match val {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(b) => Value::Bool(*b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else {
                    Value::Float(n.as_f64().unwrap())
                }
            }
            JsonValue::String(s) => Value::String(s.clone().into()),
            JsonValue::Array(arr) => {
                Value::List(arr.iter().map(Self::json_to_cel).collect::<Vec<_>>().into())
            }
            JsonValue::Object(map) => {
                let cel_map = map
                    .iter()
                    .map(|(k, v)| (Key::String(Arc::new(k.clone())), Self::json_to_cel(v)))
                    .collect::<HashMap<Key, Value>>();
                Value::Map(cel_map.into())
            }
        }
    }

    fn evaluate_assertion(expr: &str, response: &TestResult) -> anyhow::Result<bool> {
        let program = Program::compile(expr).expect("Could not compile expression");

        let mut ctx = Context::default();

        ctx.add_variable("status", Value::UInt(response.status.code as u64))?;

        if let Some(json) = &response.json {
            ctx.add_variable("json", Self::json_to_cel(json))?;
        }

        ctx.add_variable("body", &response.body)?;

        ctx.add_variable("bytes", &response.bytes)?;

        ctx.add_variable("headers", Self::headers_to_cel(&response.headers))?;

        match program.execute(&ctx)? {
            Value::Bool(b) => Ok(b),
            other => anyhow::bail!("Assertion did not return a bool, got: {:?}", other),
        }
    }

    fn headers_to_cel(headers: &HeaderMap) -> Value {
        let cel_map = headers
            .iter()
            .map(|(k, v)| {
                let key = Key::String(Arc::new(k.as_str().to_string()));
                let val = Value::String(Arc::new(v.to_str().unwrap_or("").to_string()));
                (key, val)
            })
            .collect::<HashMap<Key, Value>>();
        Value::Map(cel_map.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_result::{TempestStatusCode, TestResult};
    use serde_json::json;
    use std::time::Duration;

    fn result(status: u16, body: &str) -> TestResult {
        TestResult {
            status: TempestStatusCode {
                code: status,
                message: "OK".to_string(),
            },
            headers: reqwest::header::HeaderMap::new(),
            body: body.to_string(),
            json: None,
            bytes: vec![],
            duration: Duration::ZERO,
        }
    }

    fn with_json(r: TestResult, json: serde_json::Value) -> TestResult {
        TestResult {
            json: Some(json),
            ..r
        }
    }

    fn with_header(mut r: TestResult, name: &str, value: &str) -> TestResult {
        use reqwest::header::{HeaderName, HeaderValue};
        r.headers.insert(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
        r
    }

    fn eval(expr: &str, r: &TestResult) -> Assertion {
        CelParser::new(expr).assert(r)
    }

    #[test]
    fn status_equal_passes_and_preserves_expr() {
        let a = eval("status == 200u", &result(200, ""));
        assert!(a.passed);
        assert!(a.error.is_empty());
        assert_eq!(a.expr, "status == 200u");
    }

    #[test]
    fn status_not_equal_returns_false_with_no_error() {
        let a = eval("status == 404u", &result(200, ""));
        assert!(!a.passed);
        assert!(a.error.is_empty());
    }

    #[test]
    fn body_string_equality() {
        assert!(eval(r#"body == "hello""#, &result(200, "hello")).passed);
        assert!(!eval(r#"body == "hello""#, &result(200, "world")).passed);
    }

    #[test]
    fn body_contains_method() {
        let a = eval(r#"body.contains("ell")"#, &result(200, "hello"));
        assert!(a.passed);
    }

    #[test]
    fn json_top_level_field_access() {
        let r = with_json(result(200, ""), json!({"name": "Alice"}));
        assert!(eval(r#"json.name == "Alice""#, &r).passed);
    }

    #[test]
    fn json_nested_field_and_integer_value() {
        let r = with_json(result(200, ""), json!({"user": {"age": 30}}));
        assert!(eval("json.user.age == 30", &r).passed);
    }

    #[test]
    fn header_value_comparison() {
        let r = with_header(result(200, ""), "x-request-id", "abc-123");
        assert!(eval(r#"headers["x-request-id"] == "abc-123""#, &r).passed);
    }

    #[test]
    fn compound_and_expression() {
        let pass = eval(r#"status == 200u && body == "ok""#, &result(200, "ok"));
        assert!(pass.passed);

        let fail = eval(r#"status == 200u && body == "ok""#, &result(200, "bad"));
        assert!(!fail.passed);
        assert!(fail.error.is_empty()); // still a valid CEL expression, just false
    }

    #[test]
    fn non_bool_expression_populates_error_and_sets_passed_false() {
        let a = eval("status + 1u", &result(200, ""));
        assert!(!a.passed);
        assert!(!a.error.is_empty());
    }
}
