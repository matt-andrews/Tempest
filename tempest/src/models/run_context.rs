use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RunContext{
    pub file_name: String,
    pub file: HashMap<String, String>
}