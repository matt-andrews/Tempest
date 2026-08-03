use crate::models::run_options::RunOptions;
use crate::models::test_result::TestResult;
use crate::models::test_spec::TestSpec;
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

pub fn test_runner_for(test: &TestSpec, options: &RunOptions) -> AnyTestRunner {
    let url = resolved_route(test, options);

    HttpTestRunner::new(&url, test).into()
}

pub fn resolved_route(test: &TestSpec, options: &RunOptions) -> String {
    let mut url = test.route.trim_start().to_string();

    if !url.starts_with("http://")
        && !url.starts_with("https://")
        && let Some(base_uri) = &options.base_uri
    {
        url = format!(
            "{}/{}",
            base_uri.trim_end_matches('/'),
            url.trim_start_matches('/')
        );
    }

    url
}
