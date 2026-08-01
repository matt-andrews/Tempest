use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use cel_interpreter::Context;

pub mod file_bytes;
pub mod json;

pub fn register_all(
    context: &mut Context,
    evaluation_context: &EvaluationContext,
    response_content_cache: &ResponseContentCache,
) {
    json::register(context, response_content_cache.clone());
    file_bytes::register(context, evaluation_context.clone())
}
