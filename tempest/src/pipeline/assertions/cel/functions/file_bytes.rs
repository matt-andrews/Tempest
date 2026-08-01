use std::sync::Arc;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, ExecutionError, FunctionContext, Value};
use crate::models::assertion_context::AssertionContext;

pub fn register(context: &mut Context, assertion_context: AssertionContext){
    context.add_function(
        "fileBytes",
        move |ftx: &FunctionContext, path: Arc<String>| {
            let resolved = assertion_context
                .resolve_file(path.as_str())
                .map_err(|e| ftx.error(e.to_string()))?;

            let bytes = std::fs::read(&resolved).map_err(|e| {
                ftx.error(format!("could not read {}: {e}", resolved.display()))
            })?;

            Ok(Value::Bytes(Arc::new(bytes)))
        },
    );
}