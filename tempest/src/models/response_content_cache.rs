use cel_interpreter::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type JsonValues = HashMap<Arc<String>, Result<Value, String>>;

#[derive(Clone, Debug, Default)]
pub struct ResponseContentCache {
    pub json: Arc<Mutex<JsonValues>>,
    pub html: Arc<OnceLock<Mutex<scraper::Html>>>,
}
