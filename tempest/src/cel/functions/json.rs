use crate::content;
use crate::models::response_content_cache::ResponseContentCache;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, ExecutionError, FunctionContext, Value};
use std::sync::Arc;

pub fn register(ctx: &mut Context, cache: ResponseContentCache) {
    ctx.add_function(
        "json",
        move |ftx: &FunctionContext, This(source): This<Arc<String>>| {
            cache
                .json
                .get_or_init(|| {
                    let parsed = content::json::parse(source.as_bytes())
                        .map_err(|error| error.to_string())?;

                    cel_interpreter::to_value(parsed).map_err(|error| error.to_string())
                })
                .clone()
                .map_err(|error| ftx.error(error))
        },
    );
}
