use std::time::Instant;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use crate::engine::runner::capabilities::RunnerCapability;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::run_result::{HttpResult, RunResult, TempestStatusCode};

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
    async fn run(
        &self,
        descriptor: &DescriptorModel,
        context: &RunResult,
        options: &OptionsModel
    ) -> RunResult {
        if let Some(test) = &descriptor.test{
            let verb = test.verb.clone().unwrap_or("GET".to_string()).to_uppercase();
            let mut url = test.route.clone();
            if let Some(base_uri) = &options.base_uri{
                url = format!("{}/{}", base_uri.trim_end_matches('/'), url.trim_start_matches('/'));
            }
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

                return match builder.send().await {
                    Ok(response) => {
                        let status = response.status();
                        let headers = response.headers().clone();

                        let bytes = response.bytes().await.unwrap_or_default().to_vec();
                        let duration = start.elapsed();
                        let body = String::from_utf8_lossy(&bytes).to_string();
                        let json = serde_json::from_slice(&bytes).ok();

                        RunResult {
                            http_result: HttpResult {
                                status: TempestStatusCode::from_status(status),
                                headers,
                                bytes,
                                body,
                                json,
                                duration,
                            },
                            assertions: Vec::new(),
                            stop: false,
                        }
                    },
                    Err(err) => {
                        let duration = start.elapsed();
                        RunResult {
                            http_result: HttpResult {
                                status: TempestStatusCode::from_message(err.to_string()),
                                headers: HeaderMap::new(),
                                bytes: Vec::new(),
                                body: String::new(),
                                json: None,
                                duration,
                            },
                            assertions: Vec::new(),
                            stop: false,
                        }
                    }
                }
            }
        }
        RunResult::default()
    }
}