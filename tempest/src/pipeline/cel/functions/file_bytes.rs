use crate::models::assertion_context::EvaluationContext;
use cel_interpreter::{Context, FunctionContext, Value};
use std::sync::Arc;

pub fn register(context: &mut Context, evaluation_context: EvaluationContext) {
    context.add_function(
        "fileBytes",
        move |ftx: &FunctionContext, path: Arc<String>| {
            let resolved = evaluation_context
                .resolve_file(path.as_str())
                .map_err(|e| ftx.error(e.to_string()))?;

            let bytes = std::fs::read(&resolved)
                .map_err(|e| ftx.error(format!("could not read {}: {e}", resolved.display())))?;

            Ok(Value::Bytes(Arc::new(bytes)))
        },
    );
}
