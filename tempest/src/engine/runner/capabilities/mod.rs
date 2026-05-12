use async_trait::async_trait;
use crate::engine::runner::capabilities::assert::AssertCapability;
use crate::engine::runner::capabilities::console_report::ConsoleReportCapability;
use crate::engine::runner::capabilities::http_test::HttpTestCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::run_result::RunResult;

pub mod http_test;
pub mod console_report;
pub mod assert;

#[async_trait]
pub trait TestCapability: Send + Sync{
    async fn test(
        &self,
        descriptor: &DescriptorModel,
        options: &OptionsModel
    ) -> RunResult;
}

pub async fn create_capabilities() -> Vec<Box<dyn RunnerCapability>>{
    vec![
        Box::new(HttpTestCapability::new()),
    ]
}