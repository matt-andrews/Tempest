use crate::models::assertion_context::AssertionContext;
use cel_interpreter::Context;

pub mod file_bytes;
pub mod json;

pub fn register_all(context: &mut Context, assertion_context: &AssertionContext) {
    json::register(context);
    file_bytes::register(context, assertion_context.clone())
}
