use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use crate::engine::test_capabilities::http::HttpTestCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::test_model::TestModel;
use crate::models::test_result::TestResult;

pub mod http;
pub mod console_report;
pub mod assert;

#[async_trait]
#[enum_dispatch]
pub trait TestCapability: Send + Sync{
    async fn test(&self) -> TestResult;
}

#[enum_dispatch(TestCapability)]
pub enum TestCapabilityProvider{
    HttpTestCapability
}

pub fn get_test_capability(
    descriptor: &DescriptorModel,
    options: &OptionsModel,
) -> Option<TestCapabilityProvider>{
    if let Some(test) = &descriptor.test {
        let mut url = test.route.clone();
        if let Some(base_uri) = &options.base_uri {
            url = format!("{}/{}", base_uri.trim_end_matches('/'), url.trim_start_matches('/'));
        }

        return Some(HttpTestCapability::new(url, descriptor.clone(), options.clone(), test.clone()).into())
    }
    None
}
