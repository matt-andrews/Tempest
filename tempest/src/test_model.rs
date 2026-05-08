use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DescribeModel{
    pub tests: Vec<TestModel>,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestModel{
    pub route: String,
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub method: Option<HttpMethod>,
    pub description: Option<String>,
    pub assert: Option<Vec<AssertionModel>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AssertionModel{
    pub name: Option<String>,
    pub expr: String,
}