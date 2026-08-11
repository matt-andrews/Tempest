use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::models::run_options::RunOptions;
use crate::templating::liquid::LiquidEngine;
use crate::templating::TemplateEngine;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Project {
    pub name: String,
    pub version: Option<String>,
    pub options: Option<RunOptions>,
    pub env: Option<HashMap<String, String>>,
    pub include: Option<Vec<PathBuf>>,

    pub warn_as_err: Option<bool>,
    pub success_exit: Option<i32>,
    pub flaky_exit: Option<i32>,
    pub failed_exit: Option<i32>,
}

impl Project {
    pub fn new_with_defaults() -> Self {
        Self {
            success_exit: Some(0),
            flaky_exit: Some(0),
            failed_exit: Some(1),
            ..Project::default()
        }
    }
    pub fn merge_env(&mut self, parent_env: HashMap<String, String>){
        let mut parent_env = parent_env;
        if let Some(env) = &self.env {
            for (k, v) in env{
                parent_env.insert(k.clone(), v.clone());
            }
        }
        self.env = Some(parent_env);
    }
    pub fn render_template(&mut self, engine: &LiquidEngine){
        if let Some(env) = &self.env {
            let obj = &liquid::object!({
                "env": &env,
            });
            self.name = engine.render_string_or_self(&self.name, obj);
            self.version = engine.render_option_string_or_self(&self.version, obj);
        }
    }
}