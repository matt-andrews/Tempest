use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TestModel{
    pub route: String,
    
    pub verb: Option<String>,
    pub body: Option<String>,
    pub assert: Option<Vec<String>>,
    pub query: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
}