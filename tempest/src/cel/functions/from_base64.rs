use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, FunctionContext, Value};
use std::sync::Arc;

pub fn register(context: &mut Context) {
    context.add_function(
        "fromBase64",
        move |_ftx: &FunctionContext, This(source): This<Arc<String>>| {
            let bytes: Vec<u8> = BASE64_STANDARD.decode(source.as_str()).unwrap_or_default();
            Ok(Value::Bytes(Arc::new(bytes)))
        },
    );
}
