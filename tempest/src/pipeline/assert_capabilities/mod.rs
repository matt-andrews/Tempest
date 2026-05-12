use enum_dispatch::enum_dispatch;
use crate::pipeline::assert_capabilities::cel_parser::CelParser;
use crate::models::test_result::{Assertion, TestResult};

pub mod cel_parser;

#[enum_dispatch]
pub trait AssertCapability{
    fn assert(&self, data: &TestResult) -> Assertion;
}

#[enum_dispatch(AssertCapability)]
pub enum AssertCapabilityProvider{
    CelParser
}

pub fn get_assert_capability(assertion: String) -> AssertCapabilityProvider {
    CelParser::new(assertion).into()
}