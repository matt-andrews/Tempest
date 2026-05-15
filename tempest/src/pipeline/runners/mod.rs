use crate::models::options_model::OptionsModel;
use crate::models::test_model::TestModel;
use crate::models::test_result::TestResult;
use crate::pipeline::runners::http::HttpTestRunner;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;

pub mod http;

#[async_trait]
#[enum_dispatch]
pub trait TestRunner: Send + Sync {
    async fn run(&self) -> TestResult;
}

#[enum_dispatch(TestRunner)]
pub enum AnyTestRunner {
    HttpTestRunner,
}

pub fn test_runner_for(test: &TestModel, options: &OptionsModel) -> AnyTestRunner {
    let mut url = test.route.clone();
    if let Some(base_uri) = &options.base_uri {
        url = format!(
            "{}/{}",
            base_uri.trim_end_matches('/'),
            url.trim_start_matches('/')
        );
    }

    HttpTestRunner::new(&url, options, test).into()
}
