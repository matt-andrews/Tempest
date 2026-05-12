use std::collections::HashMap;
use cel_interpreter::{Context, Program, Value};
use cel_interpreter::objects::Key;
use reqwest::header::HeaderMap;
use serde_json::{Value as JsonValue};
use std::sync::Arc;
use crate::engine::assert_capabilities::AssertCapability;
use crate::models::test_result::{Assertion, TestResult};

pub struct CelParser{
    assertion: String
}
impl AssertCapability for CelParser{
    fn assert(&self, data: &TestResult) -> Assertion {
        let result = Self::evaluate_assertion(&self.assertion, data)
            .map_err(|e| e.to_string());
        let error = match &result{
            Ok(_) => String::new(),
            Err(e) => e.clone()
        };
        Assertion{
            expr: self.assertion.clone(),
            error,
            passed: result.unwrap_or(false),
        }
    }
}
impl CelParser{
    pub fn new(assertion: String) -> Self{
        Self{
            assertion
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

        if let Some(json) = &response.json{
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
                let val = Value::String(Arc::new(
                    v.to_str().unwrap_or("").to_string()
                ));
                (key, val)
            })
            .collect::<HashMap<Key, Value>>();
        Value::Map(cel_map.into())
    }
}