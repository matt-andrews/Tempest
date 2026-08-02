use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use cel_interpreter::Context;

pub mod css;
pub mod file_bytes;
pub mod from_base64;
pub mod json;
pub mod to_base64;
pub mod xpath;

pub fn register_all(
    context: &mut Context,
    evaluation_context: &EvaluationContext,
    response_content_cache: &ResponseContentCache,
) {
    json::register(context, response_content_cache.clone());
    file_bytes::register(context, evaluation_context.clone());
    css::register(context, response_content_cache.clone());
    xpath::register(context);
    to_base64::register(context);
    from_base64::register(context);
}
