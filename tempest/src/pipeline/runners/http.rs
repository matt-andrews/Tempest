use crate::models::options_model::OptionsModel;
use crate::models::test_model::TestModel;
use crate::models::test_result::{TempestStatusCode, TestResult};
use crate::pipeline::test_capabilities::TestCapability;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use std::sync::LazyLock;
use std::time::Instant;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("Tempest/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build HTTP client")
});

pub struct HttpTestCapability {
    route: String,
    #[allow(dead_code)] //we will need this field here sooner or later
    options: OptionsModel,
    test: TestModel,
}
impl HttpTestCapability {
    pub fn new(route: &str, options: &OptionsModel, test: &TestModel) -> Self {
        Self {
            route: route.to_string(),
            options: options.clone(),
            test: test.clone(),
        }
    }
}
#[async_trait]
impl TestCapability for HttpTestCapability {
    async fn test(&self) -> TestResult {
        let verb = self
            .test
            .verb
            .clone()
            .unwrap_or("GET".to_string())
            .to_uppercase();

        let body = self.test.body.clone().unwrap_or_default();

        let builder = match verb.as_str() {
            "GET" => Some(CLIENT.get(&self.route)),
            "POST" => Some(CLIENT.post(&self.route).body(body)),
            "PUT" => Some(CLIENT.put(&self.route).body(body)),
            "PATCH" => Some(CLIENT.patch(&self.route).body(body)),
            "DELETE" => Some(CLIENT.delete(&self.route)),
            "HEAD" => Some(CLIENT.head(&self.route)),
            _ => None,
        };

        if let Some(mut builder) = builder {
            for header in self.test.headers.clone().unwrap_or_default() {
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
                }
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
            };
        }
        TestResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::options_model::OptionsModel;
    use crate::models::test_model::TestModel;
    use crate::pipeline::test_capabilities::TestCapability;
    use mockito::Server;
    use std::collections::HashMap;

    fn capability(url: &str, test: &TestModel) -> HttpTestCapability {
        HttpTestCapability::new(url, &OptionsModel::default(), test)
    }

    fn test_model(route: &str, verb: Option<&str>) -> TestModel {
        TestModel {
            route: route.to_string(),
            verb: verb.map(str::to_string),
            ..Default::default()
        }
    }

    // --- verb routing ---

    #[tokio::test]
    async fn get_returns_status_and_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/ping")
            .with_status(200)
            .with_body("pong")
            .create_async()
            .await;

        let result = capability(
            &format!("{}/ping", server.url()),
            &test_model("/ping", Some("GET")),
        )
        .test()
        .await;

        assert_eq!(result.status.code, 200);
        assert_eq!(result.body, "pong");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn verb_defaults_to_get_when_not_set() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/default")
            .with_status(200)
            .create_async()
            .await;

        let result = capability(
            &format!("{}/default", server.url()),
            &test_model("/default", None),
        )
        .test()
        .await;

        assert_eq!(result.status.code, 200);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn post_sends_body_and_receives_response() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/submit")
            .match_body("hello body")
            .with_status(201)
            .create_async()
            .await;

        let test = TestModel {
            route: "/submit".to_string(),
            verb: Some("POST".to_string()),
            body: Some("hello body".to_string()),
            ..Default::default()
        };
        let result = capability(&format!("{}/submit", server.url()), &test)
            .test()
            .await;

        assert_eq!(result.status.code, 201);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn put_sends_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/item/1")
            .match_body("updated")
            .with_status(200)
            .create_async()
            .await;

        let test = TestModel {
            route: "/item/1".to_string(),
            verb: Some("PUT".to_string()),
            body: Some("updated".to_string()),
            ..Default::default()
        };
        let result = capability(&format!("{}/item/1", server.url()), &test)
            .test()
            .await;

        assert_eq!(result.status.code, 200);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn delete_returns_expected_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/item/1")
            .with_status(204)
            .create_async()
            .await;

        let result = capability(
            &format!("{}/item/1", server.url()),
            &test_model("/item/1", Some("DELETE")),
        )
        .test()
        .await;

        assert_eq!(result.status.code, 204);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn unknown_verb_returns_default_result_without_sending_request() {
        // No mock server needed — no request should be made.
        let test = TestModel {
            route: "http://127.0.0.1:1".to_string(),
            verb: Some("FOOBAR".to_string()),
            ..Default::default()
        };
        let cap = HttpTestCapability::new("http://127.0.0.1:1", &OptionsModel::default(), &test);
        let result = cap.test().await;

        assert_eq!(
            result.status.code, 0,
            "unknown verb should yield zero status, not a network attempt"
        );
        assert!(result.body.is_empty());
    }

    // --- headers ---

    #[tokio::test]
    async fn custom_request_headers_are_forwarded() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/headers")
            .match_header("x-api-key", "secret")
            .match_header("accept", "application/json")
            .with_status(200)
            .create_async()
            .await;

        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "secret".to_string());
        headers.insert("accept".to_string(), "application/json".to_string());

        let test = TestModel {
            route: "/headers".to_string(),
            verb: Some("GET".to_string()),
            headers: Some(headers),
            ..Default::default()
        };
        let result = capability(&format!("{}/headers", server.url()), &test)
            .test()
            .await;

        assert_eq!(result.status.code, 200);
        mock.assert_async().await;
    }

    // --- response parsing ---

    #[tokio::test]
    async fn json_response_is_parsed_into_json_field() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/data")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"key":"value"}"#)
            .create_async()
            .await;

        let result = capability(
            &format!("{}/data", server.url()),
            &test_model("/data", Some("GET")),
        )
        .test()
        .await;

        assert!(
            result.json.is_some(),
            "json field should be populated for valid JSON bodies"
        );
        assert_eq!(result.json.unwrap()["key"], "value");
    }

    #[tokio::test]
    async fn non_json_response_leaves_json_field_none() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/text")
            .with_status(200)
            .with_body("plain text")
            .create_async()
            .await;

        let result = capability(
            &format!("{}/text", server.url()),
            &test_model("/text", Some("GET")),
        )
        .test()
        .await;

        assert!(result.json.is_none());
        assert_eq!(result.body, "plain text");
    }

    #[tokio::test]
    async fn response_headers_are_captured() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/hdr")
            .with_status(200)
            .with_header("x-custom", "tempest")
            .create_async()
            .await;

        let result = capability(
            &format!("{}/hdr", server.url()),
            &test_model("/hdr", Some("GET")),
        )
        .test()
        .await;

        assert!(result.headers.contains_key("x-custom"));
    }

    // --- error handling ---

    #[tokio::test]
    async fn connection_error_produces_504_status() {
        // Port 1 is reserved and will always be refused.
        let test = test_model("http://127.0.0.1:1/nope", Some("GET"));
        let cap =
            HttpTestCapability::new("http://127.0.0.1:1/nope", &OptionsModel::default(), &test);
        let result = cap.test().await;

        assert_eq!(result.status.code, 504);
        assert!(!result.status.message.is_empty());
    }

    // --- timing ---

    #[tokio::test]
    async fn duration_is_measured_for_successful_requests() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/slow")
            .with_status(200)
            .create_async()
            .await;

        let result = capability(
            &format!("{}/slow", server.url()),
            &test_model("/slow", Some("GET")),
        )
        .test()
        .await;

        assert!(
            result.duration.as_nanos() > 0,
            "duration should be non-zero for a real HTTP round trip"
        );
    }
}
