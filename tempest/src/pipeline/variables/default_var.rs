use std::collections::HashMap;
use cel_interpreter::Value;
use crate::models::evaluation_context::EvaluationContext;
use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::cel;
use crate::pipeline::templating::TemplateEngine;
use crate::pipeline::templating::liquid::LiquidEngine;
use crate::pipeline::variables::VariableAssignment;
use serde_json::Value as JsonValue;

pub struct DefaultVariableAssignment {
    vars: HashMap<String, String>,
    liquid: LiquidEngine,
}

impl DefaultVariableAssignment {
    pub fn new(vars: &HashMap<String, String>) -> Self {
        Self {
            vars: vars.to_owned(),
            liquid: LiquidEngine,
        }
    }
}
impl VariableAssignment for DefaultVariableAssignment {
    fn set(&self, data: &TestResult, context: &mut RunContext, evaluation_context: &EvaluationContext) {
        for (k, v) in &self.vars {
            let val = match cel::evaluate(v, data, evaluation_context){
                Ok(v) => v.json().unwrap_or_default(),
                _ => JsonValue::Null
            };
            context.file.insert(k.clone(), val);
        }
    }
}
