use crate::models::options_model::OptionsModel;
use crate::models::test_model::TestModel;
use crate::models::test_result::TestResult;
use crate::pipeline::test_capabilities::http::HttpTestCapability;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;

pub mod http;

#[async_trait]
#[enum_dispatch]
pub trait TestCapability: Send + Sync {
    async fn test(&self) -> TestResult;
}

#[enum_dispatch(TestCapability)]
pub enum TestCapabilityProvider {
    HttpTestCapability,
}

pub fn get_test_capability(test: &TestModel, options: &OptionsModel) -> TestCapabilityProvider {
    let mut url = test.route.clone();
    if let Some(base_uri) = &options.base_uri {
        url = format!(
            "{}/{}",
            base_uri.trim_end_matches('/'),
            url.trim_start_matches('/')
        );
    }

    HttpTestCapability::new(&url, options, test).into()
}
