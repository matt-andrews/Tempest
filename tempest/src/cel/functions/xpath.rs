use crate::content;
use crate::content::xml::XPathValue;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, ExecutionError, FunctionContext, Value};
use std::sync::Arc;

fn xpath(
    ftx: &FunctionContext,
    This(source): This<Arc<String>>,
    expression: Arc<String>,
) -> Result<Value, ExecutionError> {
    let result = content::xml::evaluate(source.as_bytes(), expression.as_str())
        .map_err(|error| ftx.error(error))?;

    match result {
        XPathValue::Boolean(value) => Ok(Value::Bool(value)),
        XPathValue::Number(value) => Ok(Value::Float(value)),
        XPathValue::String(value) => Ok(Value::String(Arc::new(value))),
        XPathValue::Nodes(value) => {
            cel_interpreter::to_value(value).map_err(|error| ftx.error(error))
        }
    }
}

pub fn register(context: &mut Context) {
    context.add_function("xpath", xpath);
}
