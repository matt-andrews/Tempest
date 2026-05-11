pub mod capabilities;

use async_trait::async_trait;
use crate::engine::runner::capabilities::assert::AssertCapability;
use crate::engine::runner::capabilities::console_report::ConsoleReportCapability;
use crate::engine::runner::capabilities::http_test::HttpTestCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::run_result::RunResult;

#[async_trait]
pub trait RunnerCapability: Send + Sync{
    async fn run(&self, descriptor: &DescriptorModel, context: Option<RunResult>) -> RunResult;
}

pub async fn create_capabilities() -> Vec<Box<dyn RunnerCapability>>{
    vec![
        Box::new(HttpTestCapability::new()),
        Box::new(AssertCapability),
        Box::new(ConsoleReportCapability),
    ]
}