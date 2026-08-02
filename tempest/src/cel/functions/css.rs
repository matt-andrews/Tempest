use crate::content;
use crate::models::response_content_cache::ResponseContentCache;
use cel_interpreter::extractors::This;
use cel_interpreter::{Context, FunctionContext};
use std::sync::{Arc, Mutex};

pub fn register(context: &mut Context, cache: ResponseContentCache) {
    context.add_function(
        "css",
        move |ftx: &FunctionContext, This(source): This<Arc<String>>, selector: Arc<String>| {
            let document = cache
                .html
                .get_or_init(|| Mutex::new(content::html::parse(source.as_str())));

            let document = document
                .lock()
                .map_err(|_| ftx.error("cached HTML document lock was poisoned"))?;

            let matches = content::html::select(&document, selector.as_str())
                .map_err(|error| ftx.error(error))?;

            cel_interpreter::to_value(matches).map_err(|error| ftx.error(error))
        },
    );
}
