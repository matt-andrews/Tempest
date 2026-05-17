use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunContext {
    pub file_name: String,
    pub file: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub retry_attempts: usize,
}

impl RunContext {
    pub fn new(file_name: &str, env: &HashMap<String, String>) -> Self {
        Self {
            file_name: file_name.to_owned(),
            file: HashMap::new(),
            env: env.to_owned(),
            retry_attempts: 0,
        }
    }
}
