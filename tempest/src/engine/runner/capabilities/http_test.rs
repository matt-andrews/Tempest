use std::time::Instant;
use async_trait::async_trait;
use crate::engine::runner::{RunResult, RunnerCapability};
use crate::models::descriptor_model::DescriptorModel;
use crate::models::run_result::HttpResult;

pub struct HttpTestCapability{
    client: reqwest::Client
}
impl HttpTestCapability{
    pub fn new() -> Self{
        Self{
            client: reqwest::Client::new()
        }
    }
}
#[async_trait]
impl RunnerCapability for HttpTestCapability {
    async fn run(&self, descriptor: &DescriptorModel, context: Option<RunResult>) -> RunResult {
        if let Some(test) = &descriptor.test{
            let verb = test.verb.clone().unwrap_or("GET".to_string()).to_uppercase();
            let url = &test.route;
            let body = test.body.clone().unwrap_or_default();

            let builder = match verb.as_str() {
                "GET" => Some(self.client.get(url)),
                "POST" => Some(self.client.post(url).body(body)),
                "PUT" => Some(self.client.put(url).body(body)),
                "PATCH" => Some(self.client.patch(url).body(body)),
                "DELETE" => Some(self.client.delete(url)),
                "HEAD" => Some(self.client.delete(url)),
                "OPTIONS" => Some(self.client.delete(url)),
                _ => None
            };

            if let Some(mut builder) = builder{
                for header in test.headers.clone().unwrap_or_default(){
                    builder = builder.header(header.0, header.1);
                }
                let start = Instant::now();
                if let Ok(response) = builder.send().await {
                    let status = response.status();
                    let headers = response.headers().clone();

                    let bytes = response.bytes().await.unwrap_or_default().to_vec();
                    let duration = start.elapsed();
                    let body = String::from_utf8_lossy(&bytes).to_string();
                    let json = serde_json::from_slice(&bytes).ok();

                    return RunResult {
                        success: true,
                        http_result: Some(HttpResult {
                            status,
                            headers,
                            bytes,
                            body,
                            json,
                            duration,
                        }),
                        assertions: None,
                    };
                }
            }
        }
        RunResult{
            success: true,
            http_result: None,
            assertions: None,
        }
    }
}