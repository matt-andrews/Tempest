use crate::models::assertion_context::EvaluationContext;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assertions::cel::CelAssertionEvaluator;
use enum_dispatch::enum_dispatch;

pub mod cel;

#[enum_dispatch]
pub trait AssertionEvaluator {
    fn evaluate(&self, data: &TestResult, context: &EvaluationContext) -> Assertion;
}

#[enum_dispatch(AssertionEvaluator)]
pub enum AnyAssertionEvaluator {
    CelAssertionEvaluator,
}

pub fn assertion_evaluator_for(assertion: &str) -> AnyAssertionEvaluator {
    CelAssertionEvaluator::new(assertion).into()
}
