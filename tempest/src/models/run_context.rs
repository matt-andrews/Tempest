use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunContext{
    pub file_name: String,
    pub file: HashMap<String, String>
}