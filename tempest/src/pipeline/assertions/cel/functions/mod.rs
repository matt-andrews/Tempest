use cel_interpreter::Context;
use crate::models::assertion_context::AssertionContext;

pub mod json;
pub mod file_bytes;

pub fn register_all(
    context: &mut Context,
    assertion_context: &AssertionContext,
) {
    json::register(context);
    file_bytes::register(context, assertion_context.clone())
}
