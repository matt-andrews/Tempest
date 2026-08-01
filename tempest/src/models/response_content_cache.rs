use cel_interpreter::Value;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug, Default)]
pub struct ResponseContentCache {
    pub json: Arc<OnceLock<Result<Value, String>>>,
}
