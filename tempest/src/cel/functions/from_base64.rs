use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, FunctionContext, Value};
use std::sync::Arc;

pub fn register(context: &mut Context) {
    context.add_function(
        "fromBase64",
        move |ftx: &FunctionContext, This(source): This<Arc<String>>| {
            let bytes = BASE64_STANDARD
                .decode(source.as_str())
                .map_err(|e| ftx.error(format!("invalid base64: {e}")))?;
            Ok(Value::Bytes(Arc::new(bytes)))
        },
    );
}
