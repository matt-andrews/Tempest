use anyhow::anyhow;
use cel_interpreter::{Program, Value};
use crate::models::evaluation_context::EvaluationContext;
use crate::models::test_result::TestResult;

pub mod context;
pub mod functions;

pub fn evaluate(
    expression: &str,
    response: &TestResult,
    context: &EvaluationContext,
) -> anyhow::Result<Value> {
    let program = match Program::compile(expression) {
        Ok(p) => p,
        Err(e) => return Err(anyhow!("{}", e)),
    };
    let references = program.references();
    let context = context::for_response(response, context, &references)?;

    Ok(program.execute(&context)?)
}