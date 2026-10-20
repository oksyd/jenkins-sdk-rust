//! Contracts at the SDK/reqx boundary, exercised with both transport modes.
use http::{HeaderMap, HeaderValue};
use jenkins_sdk::reqx::prelude::RetryPolicy;
use jenkins_sdk::{
    Auth, Builder, DecodeError, Error, Request,
    api::{builds, jobs, queue, views},
    reqx::{
        self,
        advanced::{Interceptor, RequestContext},
    },
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

#[derive(Clone, Copy)]
enum Mode {
    #[cfg(feature = "async")]
    Async,
    #[cfg(feature = "blocking")]
    Blocking,
}

fn modes() -> Vec<Mode> {
    vec![
        #[cfg(feature = "async")]
        Mode::Async,
        #[cfg(feature = "blocking")]
        Mode::Blocking,
    ]
}

#[derive(Default)]
struct Attempts(Mutex<Vec<usize>>);

impl Interceptor for Attempts {
    fn on_request(&self, context: &RequestContext, headers: &mut HeaderMap) {
        self.0.lock().unwrap().push(context.attempt());
        headers.insert(
            "x-attempt",
            HeaderValue::from_str(&context.attempt().to_string()).unwrap(),
        );
    }
}

#[derive(Default)]
struct Options {
    crumb_ttl: Option<Duration>,
    deadline: Option<Duration>,
    max_body: Option<usize>,
    snippet_bytes: Option<Option<usize>>,
    attempts: Option<Arc<Attempts>>,
}

impl Options {
    fn apply<T>(self, builder: Builder<T>) -> Builder<T> {
        let mut builder = builder
            .auth(Auth::bearer("supersecret"))
            .configure_http(|http| {
                http.default_header(
                    "x-sdk-test".parse().unwrap(),
                    HeaderValue::from_static("configured"),
                )
            })
            .configure_http(|http| {
                http.retry_policy(
                    RetryPolicy::standard()
                        .max_attempts(3)
                        .base_backoff(Duration::from_millis(1))
                        .jitter_ratio(0.0),
                )
            })
            .configure_http(|http| http.total_timeout(self.deadline))
            .configure_http(|mut http| {
                http = http.metrics_enabled(true);
                if let Some(limit) = self.max_body {
                    http = http.max_response_body_bytes(limit);
                }
                if let Some(attempts) = self.attempts {
                    http = http.interceptor(attempts);
                }
                // Typed Jenkins endpoints must preserve redirect responses even
                // when the raw transport is configured to follow redirects.
                http.redirect_policy(reqx::prelude::RedirectPolicy::follow())
            });
        if let Some(ttl) = self.crumb_ttl {
            builder = builder.crumb_ttl(ttl);
        }
        if let Some(bytes) = self.snippet_bytes {
            builder = builder.error_body_limit(bytes);
        }
        builder
    }
}

#[derive(Clone)]
enum TestClient {
    #[cfg(feature = "async")]
    Async(jenkins_sdk::Client),
    #[cfg(feature = "blocking")]
    Blocking(jenkins_sdk::BlockingClient),
}

impl TestClient {
    fn new(mode: Mode, url: &str, options: Options) -> Self {
        match mode {
            #[cfg(feature = "async")]
            Mode::Async => Self::Async(
                options
                    .apply(jenkins_sdk::Client::builder(url).unwrap())
                    .build()
                    .unwrap(),
            ),
            #[cfg(feature = "blocking")]
            Mode::Blocking => Self::Blocking(
                options
                    .apply(jenkins_sdk::BlockingClient::builder(url).unwrap())
                    .build()
                    .unwrap(),
            ),
        }
    }

    async fn execute<T: Send + 'static>(&self, request: Request<T>) -> Result<T, Error> {
        match self {
            #[cfg(feature = "async")]
            Self::Async(client) => client.execute(request).await,
            #[cfg(feature = "blocking")]
            Self::Blocking(client) => {
                let client = client.clone();
                tokio::task::spawn_blocking(move || client.execute(request))
                    .await
                    .unwrap()
            }
        }
    }

    async fn call(&self, mutation: bool) -> Result<(), Error> {
        if mutation {
            self.execute(builds::stop("demo", builds::BuildSelector::Number(1))?)
                .await
        } else {
            self.execute(queue::list()).await.map(|_| ())
        }
    }

    fn retries(&self) -> u64 {
        match self {
            #[cfg(feature = "async")]
            Self::Async(client) => client.http_client().metrics_snapshot().requests.retries,
            #[cfg(feature = "blocking")]
            Self::Blocking(client) => client.http_client().metrics_snapshot().requests.retries,
        }
    }

    async fn raw_queue(&self) -> reqx::Result<String> {
        match self {
            #[cfg(feature = "async")]
            Self::Async(client) => client
                .http_client()
                .get("/queue/api/json")
                .send()
                .await?
                .into_text(),
            #[cfg(feature = "blocking")]
            Self::Blocking(client) => {
                let client = client.clone();
                tokio::task::spawn_blocking(move || {
                    client
                        .http_client()
                        .get("/queue/api/json")
                        .send()?
                        .into_text()
                })
                .await
                .unwrap()
            }
        }
    }
}

#[tokio::test]
async fn raw_reqx_requests_share_auth_defaults_and_base_path() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jenkins/queue/api/json"))
            .and(header("authorization", "Bearer supersecret"))
            .and(header("x-sdk-test", "configured"))
            .respond_with(ResponseTemplate::new(200).set_body_string("raw-response"))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options::default(),
        );
        assert_eq!(client.raw_queue().await.unwrap(), "raw-response");
        server.verify().await;
    }
}

#[tokio::test]
async fn read_requests_do_not_fetch_crumbs() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(path("/queue/api/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"items": []})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                crumb_ttl: Some(Duration::from_secs(60)),
                ..Options::default()
            },
        );
        client.call(false).await.unwrap();
        server.verify().await;
    }
}

#[tokio::test]
async fn reqx_retries_invoke_interceptors_and_update_metrics() {
    for mode in modes() {
        let server = MockServer::start().await;
        for (attempt, status) in [(1, 503), (2, 200)] {
            Mock::given(method("GET"))
                .and(path("/jenkins/queue/api/json"))
                .and(header("x-attempt", attempt.to_string().as_str()))
                .and(header("authorization", "Bearer supersecret"))
                .and(header("x-sdk-test", "configured"))
                .respond_with(
                    ResponseTemplate::new(status).set_body_json(serde_json::json!({"items": []})),
                )
                .expect(1)
                .mount(&server)
                .await;
        }
        let attempts = Arc::new(Attempts::default());
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options {
                attempts: Some(attempts.clone()),
                ..Options::default()
            },
        );
        client.call(false).await.unwrap();
        assert_eq!(*attempts.0.lock().unwrap(), [1, 2]);
        assert_eq!(client.retries(), 1);
        server.verify().await;
    }
}

#[tokio::test]
async fn jenkins_mutations_are_not_replayed_by_default() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/job/demo/1/stop"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        assert_eq!(
            client
                .call(true)
                .await
                .unwrap_err()
                .response()
                .map(|r| r.status),
            Some(http::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(client.retries(), 0);
        server.verify().await;
    }
}

#[tokio::test]
async fn total_deadline_bounds_retry_after_waits() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(429).append_header("Retry-After", "60"))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                deadline: Some(Duration::from_millis(100)),
                ..Options::default()
            },
        );
        let started = std::time::Instant::now();
        let err = client.call(false).await.unwrap_err();
        assert!(
            matches!(err, Error::Transport(ref source) if matches!(**source, reqx::Error::DeadlineExceeded { .. })),
            "{err:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        server.verify().await;
    }
}

#[tokio::test]
async fn response_limit_preserves_native_error() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(128)))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                max_body: Some(16),
                ..Options::default()
            },
        );
        let err = client.call(false).await.unwrap_err();
        assert!(
            matches!(err, Error::Transport(ref source) if matches!(**source, reqx::Error::ResponseBodyTooLarge { .. })),
            "{err:?}"
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn truncated_error_snippets_do_not_expose_partial_tokens() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403).set_body_string("supersecret"))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                snippet_bytes: Some(Some(6)),
                ..Options::default()
            },
        );
        let err = client.call(false).await.unwrap_err();
        let Error::Http(http) = err else {
            panic!("{err:?}")
        };
        assert_eq!(http.body_snippet.as_deref(), Some("<redac"));
        server.verify().await;
    }
}

#[tokio::test]
async fn clones_share_one_crumb_fetch_and_preserve_redirects() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jenkins/crumbIssuer/api/json"))
            .and(header("authorization", "Bearer supersecret"))
            .and(header("x-sdk-test", "configured"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(20))
                    .set_body_json(serde_json::json!({
                        "crumbRequestField": "Jenkins-Crumb", "crumb": "crumb-value"
                    })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/jenkins/job/demo/1/stop"))
            .and(header("jenkins-crumb", "crumb-value"))
            .respond_with(ResponseTemplate::new(302).append_header("Location", "/must-not-follow"))
            .expect(8)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options {
                crumb_ttl: Some(Duration::from_secs(60)),
                ..Options::default()
            },
        );
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let client = client.clone();
            tasks.push(tokio::spawn(async move { client.call(true).await }));
        }
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn expired_crumbs_are_fetched_again() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "crumbRequestField": "Jenkins-Crumb", "crumb": "crumb-value"
            })))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(header("jenkins-crumb", "crumb-value"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                crumb_ttl: Some(Duration::ZERO),
                ..Options::default()
            },
        );
        client.call(true).await.unwrap();
        client.call(true).await.unwrap();
        server.verify().await;
    }
}

#[tokio::test]
async fn failed_crumb_fetch_is_not_cached_or_followed_by_a_mutation() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(403))
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                crumb_ttl: Some(Duration::from_secs(60)),
                ..Options::default()
            },
        );
        for _ in 0..2 {
            assert!(
                client
                    .call(true)
                    .await
                    .unwrap_err()
                    .response()
                    .is_some_and(|r| r.status == http::StatusCode::FORBIDDEN)
            );
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn progressive_logs_require_valid_offsets_and_utf8() {
    let cases = [
        (ResponseTemplate::new(200), "missing offset"),
        (
            ResponseTemplate::new(200).append_header("X-Text-Size", "invalid"),
            "invalid offset",
        ),
        (
            ResponseTemplate::new(200)
                .append_header("X-Text-Size", "4")
                .append_header("X-More-Data", "maybe"),
            "invalid continuation flag",
        ),
        (
            ResponseTemplate::new(200)
                .append_header("X-Text-Size", "1")
                .set_body_bytes(vec![0xff]),
            "invalid UTF-8",
        ),
    ];
    for mode in modes() {
        for (response, label) in &cases {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(response.clone())
                .expect(1)
                .mount(&server)
                .await;
            let client = TestClient::new(mode, &server.uri(), Options::default());
            let error = client
                .execute(
                    builds::progressive_console_text("demo", builds::BuildSelector::Last, 0)
                        .unwrap(),
                )
                .await
                .unwrap_err();
            assert!(matches!(error, Error::Decode { .. }), "{label}: {error:?}");
            assert_eq!(error.response().unwrap().status, 200);
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn completed_progressive_log_uses_server_byte_offset() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(path("/job/demo/lastBuild/logText/progressiveText"))
            .and(wiremock::matchers::query_param("start", "12"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("X-Text-Size", "15")
                    .set_body_string("好"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        let log = client
            .execute(
                builds::progressive_console_text("demo", builds::BuildSelector::Last, 12).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(log.text, "好");
        assert_eq!(log.next_start, 15);
        assert!(!log.more_data);
        server.verify().await;
    }
}

#[tokio::test]
async fn build_requires_a_queue_location() {
    for mode in modes() {
        for location in [
            None,
            Some("/login"),
            Some("/queue/item/not-a-number/"),
            Some("/queue/item/1/?token=secret"),
            Some("/queue/item/1//"),
            Some("file:///queue/item/1/"),
        ] {
            let server = MockServer::start().await;
            let mut response = ResponseTemplate::new(201);
            if let Some(value) = location {
                response = response.append_header("Location", value);
            }
            Mock::given(method("POST"))
                .respond_with(response)
                .expect(1)
                .mount(&server)
                .await;
            let client = TestClient::new(mode, &server.uri(), Options::default());
            let error = client
                .execute(jobs::build("demo").unwrap())
                .await
                .unwrap_err();
            assert!(
                matches!(error, Error::Decode { .. }),
                "{location:?}: {error:?}"
            );
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn parameterized_build_preserves_duplicate_form_values_and_resolves_location() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/jenkins/job/demo/buildWithParameters"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(wiremock::matchers::body_string("target=a%2Bb&target=c+d"))
            .respond_with(
                ResponseTemplate::new(201).append_header("Location", "../../queue/item/42/"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options::default(),
        );
        let result = client
            .execute(
                jobs::build_with_parameters("demo", [("target", "a+b"), ("target", "c d")])
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.queue_item_id, 42);
        assert_eq!(
            result.location.as_str(),
            format!("{}/jenkins/queue/item/42/", server.uri())
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn json_collections_and_numbers_are_not_silently_coerced() {
    for mode in modes() {
        for payload in [
            serde_json::json!({}),
            serde_json::json!({"items": [{"id": "42"}]}),
            serde_json::json!({"items": [{"executable": {"number": "12"}}]}),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(payload))
                .expect(1)
                .mount(&server)
                .await;
            let client = TestClient::new(mode, &server.uri(), Options::default());
            let error = client.execute(queue::list()).await.unwrap_err();
            assert!(matches!(
                error,
                Error::Decode {
                    source: DecodeError::Json(_),
                    ..
                }
            ));
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn tree_projection_replaces_defaults_and_preserves_absence() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(path("/api/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"jobs": []})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/view/production/api/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"name": "production"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        client
            .execute(jobs::list().tree("jobs[name]"))
            .await
            .unwrap();
        let view = client
            .execute(views::get("production").unwrap().tree("name"))
            .await
            .unwrap();
        assert_eq!(view.name.as_deref(), Some("production"));
        assert!(view.jobs.is_none());
        assert!(view.property.is_none());
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].url.query_pairs().collect::<Vec<_>>(),
            [("tree".into(), "jobs[name]".into())]
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn get_redirects_remain_http_errors_with_context() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(path("/queue/api/json"))
            .respond_with(
                ResponseTemplate::new(302)
                    .append_header("Location", "/login")
                    .append_header("X-Request-Id", "request-42")
                    .append_header("Retry-After", "120")
                    .set_body_string("private response"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &server.uri(),
            Options {
                snippet_bytes: Some(None),
                ..Options::default()
            },
        );
        let error = client
            .execute(queue::list().query("token", "private"))
            .await
            .unwrap_err();
        let Error::Http(context) = error else {
            panic!("{error:?}")
        };
        assert_eq!(context.status, 302);
        assert_eq!(context.method, http::Method::GET);
        assert_eq!(context.request_id.as_deref(), Some("request-42"));
        assert_eq!(context.retry_after, Some(Duration::from_secs(120)));
        assert!(context.body_snippet.is_none());
        assert!(context.url.query().is_none());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        server.verify().await;
    }
}

#[tokio::test]
async fn invalid_crumb_headers_prevent_mutation() {
    for mode in modes() {
        for (name, value) in [("bad name", "value"), ("Jenkins-Crumb", "bad\nvalue")] {
            let server = MockServer::start().await;
            Mock::given(path("/crumbIssuer/api/json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "crumbRequestField": name, "crumb": value
                })))
                .expect(1)
                .mount(&server)
                .await;
            let client = TestClient::new(
                mode,
                &server.uri(),
                Options {
                    crumb_ttl: Some(Duration::from_secs(60)),
                    ..Options::default()
                },
            );
            let error = client.call(true).await.unwrap_err();
            assert!(matches!(error, Error::Decode { .. }));
            assert_eq!(
                error.response().unwrap().url.path(),
                "/crumbIssuer/api/json"
            );
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn custom_requests_preserve_context_encoding_queries_and_headers() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/jenkins/job/literal%252Fname/api/xml"))
            .and(header("accept", "application/xml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("X-Jenkins", "2.580.1")
                    .set_body_string("<job/>"),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/jenkins/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("root"))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options::default(),
        );
        let response = client
            .execute(
                Request::new(http::Method::GET, ["job", "literal%2Fname", "api", "xml"])
                    .unwrap()
                    .header(
                        http::header::ACCEPT,
                        HeaderValue::from_static("application/xml"),
                    )
                    .query("exclude", "old")
                    .query("exclude", "first")
                    .append_query("exclude", "second"),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["x-jenkins"], "2.580.1");
        assert_eq!(response.body().as_ref(), b"<job/>");
        let root = client
            .execute(
                Request::new(http::Method::GET, Vec::<String>::new())
                    .unwrap()
                    .decode_text(),
            )
            .await
            .unwrap();
        assert_eq!(root, "root");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].url.query_pairs().collect::<Vec<_>>(),
            [
                ("exclude".into(), "first".into()),
                ("exclude".into(), "second".into())
            ]
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn crumb_sessions_obey_cookie_scope_and_multipart_requires_original_field_case() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(path("/jenkins/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(200)
                .append_header("Set-Cookie", "session=active; Path=/jenkins/; HttpOnly")
                .append_header("Set-Cookie", "other=hidden; Path=/elsewhere/")
                .set_body_json(serde_json::json!({"crumbRequestField": "Jenkins-Crumb", "crumb": "crumb-value"})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/jenkins/doFingerprintCheck"))
            .and(header("cookie", "session=active"))
            .and(header("Jenkins-Crumb", "crumb-value"))
            .respond_with(
                ResponseTemplate::new(302).append_header("Location", "/jenkins/fingerprint/abc/"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(
            mode,
            &format!("{}/jenkins", server.uri()),
            Options {
                crumb_ttl: Some(Duration::from_secs(60)),
                ..Options::default()
            },
        );
        client
            .execute(
                jenkins_sdk::api::fingerprints::check("data.bin", vec![0, 255, 13, 10]).unwrap(),
            )
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let upload = &requests[1];
        let content_type = upload.headers["content-type"].to_str().unwrap();
        let boundary = content_type
            .strip_prefix("multipart/form-data; boundary=")
            .unwrap();
        assert!(
            upload
                .body
                .starts_with(format!("--{boundary}\r\n").as_bytes())
        );
        assert!(
            upload
                .body
                .ends_with(format!("--{boundary}--\r\n").as_bytes())
        );
        let body = String::from_utf8_lossy(&upload.body);
        assert!(body.contains("name=\"name\"; filename=\"data.bin\""));
        assert!(body.contains("name=\"Jenkins-Crumb\"\r\n\r\ncrumb-value\r\n"));
        assert!(
            upload
                .body
                .windows(4)
                .any(|bytes| bytes == [0, 255, 13, 10])
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn management_json_errors_are_not_reported_as_success() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"status": "error", "message": "No token found"}),
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        let error = client
            .execute(jenkins_sdk::api::tokens::rename("admin", "absent", "new").unwrap())
            .await
            .unwrap_err();
        assert!(
            matches!(error, Error::Decode { source: DecodeError::Rejected(ref message), .. }
            if message == "No token found")
        );
        assert_eq!(error.response().unwrap().status, 200);
        server.verify().await;
    }
}

#[tokio::test]
async fn plugin_install_uses_json_body_and_unwraps_management_data() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/pluginManager/installPlugins"))
            .and(header("content-type", "application/json"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({"plugins": ["git.default"], "dynamicLoad": false}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"status": "ok", "data": {"correlationId": "install-1"}}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        let data = client
            .execute(jenkins_sdk::api::plugins::install(&["git.default"], false).unwrap())
            .await
            .unwrap();
        assert_eq!(data["correlationId"], "install-1");
        server.verify().await;
    }
}

#[tokio::test]
async fn setup_distinguishes_unavailable_platform_data_from_an_empty_list() {
    for mode in modes() {
        for (body, expected) in [
            (serde_json::json!({"status": "ok"}), None),
            (
                serde_json::json!({"status": "ok", "data": []}),
                Some(Vec::<serde_json::Value>::new()),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/setupWizard/platformPluginList"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            let client = TestClient::new(mode, &server.uri(), Options::default());
            assert_eq!(
                client
                    .execute(jenkins_sdk::api::setup::platform_plugins())
                    .await
                    .unwrap(),
                expected
            );
            server.verify().await;
        }
    }
}

#[cfg(feature = "compression-gzip")]
#[tokio::test]
async fn reqx_decompresses_json_when_gzip_is_enabled() {
    for mode in modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wiremock::matchers::header_regex(
                "accept-encoding",
                "(^|,\\s*)gzip(,|$)",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("Content-Encoding", "gzip")
                    .set_body_bytes(vec![
                        31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 171, 86, 202, 44, 73, 205, 45, 86, 178,
                        138, 142, 173, 5, 0, 29, 31, 67, 234, 12, 0, 0, 0,
                    ]),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = TestClient::new(mode, &server.uri(), Options::default());
        client.call(false).await.unwrap();
        server.verify().await;
    }
}
