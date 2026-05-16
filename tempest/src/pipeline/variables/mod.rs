use enum_dispatch::enum_dispatch;
use crate::models::run_context::RunContext;
use crate::models::test_result::TestResult;
use crate::pipeline::variables::default_var::{DefaultVariableAssignment};

pub mod default_var;

#[enum_dispatch]
pub trait VariableAssignment{
    fn set(&self, data: &TestResult, context: &mut RunContext);
}

#[enum_dispatch(VariableAssignment)]
pub enum AnyVariableAssignment{
    DefaultVariableAssignment
}

pub fn variable_assignment_for(var: &str) -> AnyVariableAssignment {
    DefaultVariableAssignment::new(var).into()
}