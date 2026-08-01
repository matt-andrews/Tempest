use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::variables::default_var::DefaultVariableAssignment;
use enum_dispatch::enum_dispatch;
use std::collections::HashMap;

pub mod default_var;

#[enum_dispatch]
pub trait VariableAssignment {
    fn set(
        &self,
        data: &TestResult,
        context: &mut RunContext,
        evaluation_context: &EvaluationContext,
        response_content_cache: &ResponseContentCache,
    );
}

#[enum_dispatch(VariableAssignment)]
pub enum AnyVariableAssignment {
    DefaultVariableAssignment,
}

pub fn variable_assignment_for(vars: &HashMap<String, String>) -> AnyVariableAssignment {
    DefaultVariableAssignment::new(vars).into()
}
