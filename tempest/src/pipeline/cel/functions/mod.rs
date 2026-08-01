use crate::models::evaluation_context::EvaluationContext;
use cel_interpreter::Context;

pub mod file_bytes;
pub mod json;

pub fn register_all(context: &mut Context, evaluation_context: &EvaluationContext) {
    json::register(context);
    file_bytes::register(context, evaluation_context.clone())
}
