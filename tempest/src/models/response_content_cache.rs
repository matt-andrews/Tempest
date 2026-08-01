use cel_interpreter::{ExecutionError, Value};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug, Default)]
pub struct ResponseContentCache {
    pub json: Arc<OnceLock<Result<Value, String>>>,
}
