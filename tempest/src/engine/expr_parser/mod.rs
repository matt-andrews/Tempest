use enum_dispatch::enum_dispatch;
use crate::engine::expr_parser::cel_parser::{CelParser};
use crate::models::run_result::{Assertion, HttpResult};

pub mod cel_parser;

#[enum_dispatch]
pub trait ExpressionParser{
    fn assert(&self, assertions: Vec<String>, data: &HttpResult) -> Vec<Assertion>;
}

#[enum_dispatch(ExpressionParser)]
pub enum ExpressionParserProvider{
    CelParser
}

pub fn expr_parser_provider() -> ExpressionParserProvider{
    CelParser.into()
}