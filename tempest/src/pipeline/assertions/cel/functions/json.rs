use crate::content;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, ExecutionError, FunctionContext, Value};
use std::sync::Arc;

fn json(ftx: &FunctionContext, This(source): This<Arc<String>>) -> Result<Value, ExecutionError> {
    let parsed = content::json::parse(source.as_bytes()).map_err(|error| ftx.error(error))?;

    cel_interpreter::to_value(parsed).map_err(|error| ftx.error(error))
}

pub fn register(ctx: &mut Context) {
    ctx.add_function("json", json);
}
