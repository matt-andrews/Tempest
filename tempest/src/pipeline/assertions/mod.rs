use crate::models::evaluation_context::EvaluationContext;
use crate::models::response_content_cache::ResponseContentCache;
use crate::models::test_result::{Assertion, TestResult};
use crate::pipeline::assertions::cel::CelAssertionEvaluator;
use enum_dispatch::enum_dispatch;

pub mod cel;

#[enum_dispatch]
pub trait AssertionEvaluator {
    fn evaluate(
        &self,
        data: &TestResult,
        context: &EvaluationContext,
        response_content_cache: &ResponseContentCache,
    ) -> Assertion;
}

#[enum_dispatch(AssertionEvaluator)]
pub enum AnyAssertionEvaluator {
    CelAssertionEvaluator,
}

pub fn assertion_evaluator_for(assertion: &str) -> AnyAssertionEvaluator {
    CelAssertionEvaluator::new(assertion).into()
}
