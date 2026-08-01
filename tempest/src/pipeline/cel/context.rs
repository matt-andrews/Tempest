use crate::models::assertion_context::AssertionContext;
use crate::models::test_result::TestResult;
use crate::pipeline::cel::functions;
use crate::pipeline::warnings;
use cel_interpreter::objects::Key;
use cel_interpreter::{Context, Value};
use cel_parser::ExpressionReferences;
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use std::sync::Arc;

pub fn for_response<'a>(
    response: &TestResult,
    assertion_context: &AssertionContext,
    refs: &ExpressionReferences,
) -> anyhow::Result<Context<'a>> {
    let mut context = Context::default();
    check_for_warnings(refs);
    add_response_variables(&mut context, response)?;
    functions::register_all(&mut context, assertion_context);

    Ok(context)
}

fn check_for_warnings(refs: &ExpressionReferences) {
    if refs.has_variable("json") {
        warnings::append_warning(
            "`json` property is obsolete and no longer works. Use `body.json()` instead.",
        );
    }
}

fn add_response_variables(ctx: &mut Context, response: &TestResult) -> anyhow::Result<()> {
    ctx.add_variable("status", Value::UInt(response.status.code as u64))?;

    ctx.add_variable("status_message", &response.status.message)?;

    ctx.add_variable("body", &response.body)?;

    ctx.add_variable_from_value("bytes", response.bytes.clone());

    ctx.add_variable("headers", headers_to_cel(&response.headers))?;

    //cast milliseconds down to u64 since the odds of actually needing the truncated value are very slim
    //and its easier than trying to force an u128 into Value::UInt
    ctx.add_variable(
        "duration",
        Value::UInt(response.duration.as_millis() as u64),
    )?;

    Ok(())
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
