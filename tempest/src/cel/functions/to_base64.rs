use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, FunctionContext, Value};
use std::sync::Arc;

pub fn register(context: &mut Context) {
    context.add_function(
        "toBase64",
        move |_ftx: &FunctionContext, This(source): This<Arc<Vec<u8>>>| {
            let encoded = BASE64_STANDARD.encode(source.as_slice());
            Ok(Value::String(Arc::new(encoded)))
        },
    );
}
