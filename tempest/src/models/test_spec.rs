use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct TestSpec {
    pub route: String,

    pub verb: Option<String>,
    pub body: Option<String>,
    pub assert: Option<Vec<String>>,
    pub vars: Option<HashMap<String, String>>,
    #[serde(rename = "let")]
    pub lets: Option<IndexMap<String, String>>,
    pub query: Option<HashMap<String, String>>, //todo implement
    pub headers: Option<HashMap<String, String>>,
}

impl TestSpec {
    pub fn render_template(&mut self, engine: &LiquidEngine, obj: &liquid_core::Object) {
        self.route = engine.render_string_or_self(&self.route, obj);

        self.verb = engine.render_option_string_or_self(&self.verb, obj);
        self.body = engine.render_option_string_or_self(&self.body, obj);
        self.assert = engine.render_vec_string_or_self(&self.assert, obj);
        self.vars = engine.render_hashmap_string_or_self(&self.vars, obj);
        self.lets = engine.render_indexmap_string_or_self(&self.lets, obj);
        self.query = engine.render_hashmap_string_or_self(&self.query, obj);
        self.headers = engine.render_hashmap_string_or_self(&self.headers, obj);
    }
}
