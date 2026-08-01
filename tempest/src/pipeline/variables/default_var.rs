use crate::models::evaluation_context::EvaluationContext;
use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::cel;
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
    ) {
        for (k, v) in &self.vars {
            let val = match cel::evaluate(v, data, evaluation_context) {
                Ok(v) => v.json().unwrap_or_default(),
                _ => JsonValue::Null,
            };
            context.file.insert(k.clone(), val);
        }
    }
}
