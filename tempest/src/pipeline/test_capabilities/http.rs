use std::time::Instant;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use crate::pipeline::test_capabilities::{TestCapability};
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::test_model::TestModel;
use crate::models::test_result::{TempestStatusCode, TestResult};

pub struct HttpTestCapability{
    client: reqwest::Client,
    route: String,
    descriptor: DescriptorModel,
    options: OptionsModel,
    test: TestModel,
}
impl HttpTestCapability{
    pub fn new(route: String, descriptor: DescriptorModel, options: OptionsModel, test: TestModel) -> Self{
        Self{
            client: reqwest::Client::builder()
                .user_agent(concat!("Tempest/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build HTTP client"),
            route,
            descriptor,
            options,
            test,
        }
    }
}
#[async_trait]
impl TestCapability for HttpTestCapability {
    async fn test(&self) -> TestResult {
        let verb = self.test.verb.clone().unwrap_or("GET".to_string()).to_uppercase();

        let body = self.test.body.clone().unwrap_or_default();

        let builder = match verb.as_str() {
            "GET" => Some(self.client.get(&self.route)),
            "POST" => Some(self.client.post(&self.route).body(body)),
            "PUT" => Some(self.client.put(&self.route).body(body)),
            "PATCH" => Some(self.client.patch(&self.route).body(body)),
            "DELETE" => Some(self.client.delete(&self.route)),
            "HEAD" => Some(self.client.head(&self.route)),
            _ => None
        };

        if let Some(mut builder) = builder{
            for header in self.test.headers.clone().unwrap_or_default(){
                builder = builder.header(header.0, header.1);
            }
            let start = Instant::now();

            return match builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    let headers = response.headers().clone();

                    let bytes = response.bytes().await.unwrap_or_default().to_vec();
                    let duration = start.elapsed();
                    let body = String::from_utf8_lossy(&bytes).to_string();
                    let json = serde_json::from_slice(&bytes).ok();

                    TestResult {
                        status: TempestStatusCode::from_status(status),
                        headers,
                        bytes,
                        body,
                        json,
                        duration,
                    }
                },
                Err(err) => {
                    let duration = start.elapsed();
                    TestResult {
                        status: TempestStatusCode::from_message(err.to_string()),
                        headers: HeaderMap::new(),
                        bytes: Vec::new(),
                        body: String::new(),
                        json: None,
                        duration,
                    }
                }
            }
        }
        TestResult::default()
    }
}