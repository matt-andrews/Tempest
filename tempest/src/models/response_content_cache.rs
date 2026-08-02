use cel_interpreter::Value;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug, Default)]
pub struct ResponseContentCache {
    pub json: Arc<OnceLock<Result<Value, String>>>,
    pub html: Arc<OnceLock<Mutex<scraper::Html>>>,
}
