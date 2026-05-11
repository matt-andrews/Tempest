use async_trait::async_trait;
use crate::engine::expr_parser::{expr_parser_provider, ExpressionParser};
use crate::engine::runner::capabilities::RunnerCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::run_result::RunResult;

pub struct AssertCapability;
#[async_trait]
impl RunnerCapability for AssertCapability {
    async fn run(&self, descriptor: &DescriptorModel, context: &RunResult) -> RunResult {
        let http_result = &context.http_result;

        if let Some(test) = &descriptor.test{
            let expr_parser = expr_parser_provider();
            let assert = expr_parser.assert(test.assert.clone().unwrap_or_default(), http_result);
            return RunResult{
                stop: false,
                assertions: assert,
                http_result: context.http_result.clone(),
            };
        }

        RunResult::default()
    }
}