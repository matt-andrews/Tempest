use crate::content;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, ExecutionError, FunctionContext, Value};
use std::sync::Arc;

fn css(
    ftx: &FunctionContext,
    This(source): This<Arc<String>>,
    selector: Arc<String>,
) -> Result<Value, ExecutionError> {
    let matches = content::html::select(source.as_str(), selector.as_str())
        .map_err(|error| ftx.error(error))?;

    cel_interpreter::to_value(matches).map_err(|error| ftx.error(error))
}

pub fn register(context: &mut Context) {
    context.add_function("css", css);
}
