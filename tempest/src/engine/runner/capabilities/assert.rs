use async_trait::async_trait;
use crate::engine::expr_parser;
use crate::engine::expr_parser::ExpressionParser;
use crate::engine::runner::RunnerCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::run_result::RunResult;

pub struct AssertCapability;
#[async_trait]
impl RunnerCapability for AssertCapability {
    async fn run(&self, descriptor: &DescriptorModel, context: Option<RunResult>) -> RunResult {
        if let Some(mut context) = context{
            if let Some(http_result) = &context.http_result{
                if let Some(test) = &descriptor.test{
                    let expr_parser = expr_parser::expr_parser_provider();
                    context.assertions = Some(expr_parser.assert(test.assert.clone().unwrap_or_default(), http_result));
                }
            }
            return context.clone();
        }
        RunResult{
            success: true,
            http_result: None,
            assertions: None,
        }
    }
}