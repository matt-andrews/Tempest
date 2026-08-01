use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::test_result::TestResult;
use anyhow::anyhow;
use cel_interpreter::{Program, Value};

pub mod context;
pub mod functions;

pub fn evaluate(
    expression: &str,
    response: &TestResult,
    context: &EvaluationContext,
    response_content_cache: &ResponseContentCache,
) -> anyhow::Result<Value> {
    let program = match Program::compile(expression) {
        Ok(p) => p,
        Err(e) => return Err(anyhow!("{}", e)),
    };
    let references = program.references();
    let context = context::for_response(response, context, &references, response_content_cache)?;

    Ok(program.execute(&context)?)
}
