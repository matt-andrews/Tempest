use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct TestSpec {
    pub route: String,

    pub verb: Option<String>,
    pub body: Option<String>,
    pub assert: Option<Vec<String>>,
    pub vars: Option<Vec<String>>,
    pub query: Option<HashMap<String, String>>, //todo implement
    pub headers: Option<HashMap<String, String>>,
}
