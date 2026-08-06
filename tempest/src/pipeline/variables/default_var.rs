use crate::cel::{self, LetBindings};
use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::variables::VariableAssignment;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub struct DefaultVariableAssignment {
    vars: HashMap<String, String>,
}

impl DefaultVariableAssignment {
    pub fn new(vars: &HashMap<String, String>) -> Self {
        Self {
            vars: vars.to_owned(),
        }
    }
}
impl VariableAssignment for DefaultVariableAssignment {
    fn set(
        &self,
        data: &TestResult,
        context: &mut RunContext,
        evaluation_context: &EvaluationContext,
        response_content_cache: &ResponseContentCache,
        let_bindings: &LetBindings,
    ) {
        for (k, v) in &self.vars {
            let val = match cel::evaluate(
                v,
                data,
                evaluation_context,
                response_content_cache,
                let_bindings,
            ) {
                Ok(v) => v.json().unwrap_or_default(),
                _ => JsonValue::Null,
            };
            context.set_var(k.clone(), val);
        }
    }
}
