mod fixture {
    use std::time::Duration;

    use anyhow::Result;
    use serde_json::json;
    use wiremock::{
        Match, Mock, MockServer, Request, ResponseTemplate,
        matchers::{body_string_contains, header, method, path},
    };

    #[cfg(feature = "blocking")]
    use jenkins_sdk::BlockingClient;
    #[cfg(feature = "async")]
    use jenkins_sdk::Client;

    pub(crate) const TEST_USER: &str = "user";
    pub(crate) const TEST_TOKEN: &str = "token";
    pub(crate) const TEST_BASIC_AUTH: &str = "Basic dXNlcjp0b2tlbg==";

    #[derive(Clone, Copy)]
    struct CrumbHeader(&'static str);

    impl Match for CrumbHeader {
        fn matches(&self, request: &Request) -> bool {
            request
                .headers
                .get("Jenkins-Crumb")
                .and_then(|value| value.to_str().ok())
                .map(|value| value == self.0)
                .unwrap_or(false)
        }
    }

    pub(crate) async fn mock_crumb(server: &MockServer, crumb: &'static str, expected: u64) {
        Mock::given(method("GET"))
            .and(path("/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "crumbRequestField": "Jenkins-Crumb",
                "crumb": crumb
            })))
            .expect(expected)
            .up_to_n_times(expected)
            .mount(server)
            .await;
    }

    #[cfg(feature = "async")]
    pub(crate) async fn mock_get(
        server: &MockServer,
        endpoint: &str,
        response: ResponseTemplate,
        expected: u64,
    ) {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(response)
            .expect(expected)
            .up_to_n_times(expected)
            .mount(server)
            .await;
    }

    pub(crate) async fn mock_post_with_auth(
        server: &MockServer,
        endpoint: &str,
        crumb: Option<&'static str>,
        body_snippet: Option<&'static str>,
        response: ResponseTemplate,
        expected: u64,
    ) {
        let mut mock = Mock::given(method("POST"))
            .and(path(endpoint))
            .and(header("Authorization", TEST_BASIC_AUTH));

        if let Some(token) = crumb {
            mock = mock.and(CrumbHeader(token));
        }

        if let Some(snippet) = body_snippet {
            mock = mock.and(body_string_contains(snippet));
        }

        mock.respond_with(response)
            .expect(expected)
            .up_to_n_times(expected)
            .mount(server)
            .await;
    }

    #[cfg(feature = "async")]
    pub(crate) fn build_async_client(base_url: impl Into<String>) -> Result<Client> {
        Ok(Client::builder(base_url.into())?.build()?)
    }

    #[cfg(feature = "async")]
    pub(crate) fn build_async_authed_client(
        base_url: impl Into<String>,
        crumb_ttl: Duration,
    ) -> Result<Client> {
        Ok(Client::builder(base_url.into())?
            .auth(jenkins_sdk::Auth::basic(TEST_USER, TEST_TOKEN))
            .crumb_ttl(crumb_ttl)
            .build()?)
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn build_blocking_client(base_url: impl Into<String>) -> Result<BlockingClient> {
        Ok(BlockingClient::builder(base_url.into())?.build()?)
    }

    #[cfg(feature = "blocking")]
    pub(crate) fn build_blocking_authed_client(
        base_url: impl Into<String>,
        crumb_ttl: Duration,
    ) -> Result<BlockingClient> {
        Ok(BlockingClient::builder(base_url.into())?
            .auth(jenkins_sdk::Auth::basic(TEST_USER, TEST_TOKEN))
            .crumb_ttl(crumb_ttl)
            .build()?)
    }

    #[cfg(feature = "blocking")]
    pub(crate) async fn run_blocking<T, F>(f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        tokio::task::spawn_blocking(f).await?
    }
}

#[cfg(feature = "async")]
mod async_computers {
    use anyhow::Result;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, header, method, path, query_param},
    };

    use crate::fixture::build_async_client;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_computers_create_from_xml_posts_xml() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/computer/createItem"))
            .and(query_param("name", "agent-1"))
            .and(header("Content-Type", "application/xml"))
            .and(body_string_contains("<slave/>"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = build_async_client(server.uri())?;
        client
            .execute(jenkins_sdk::api::computers::create_from_xml(
                "agent-1", "<slave/>",
            )?)
            .await?;

        server.verify().await;
        Ok(())
    }
}

#[cfg(feature = "async")]
mod async_core {
    use std::time::Duration;

    use anyhow::Result;
    use http::StatusCode;
    use jenkins_sdk::{Client, Error};
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use crate::fixture::{
        TEST_USER, build_async_authed_client, build_async_client, mock_crumb, mock_get,
        mock_post_with_auth,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_client_refreshes_expired_crumb() -> Result<()> {
        let server = MockServer::start().await;

        mock_crumb(&server, "token-1", 1).await;
        mock_crumb(&server, "token-2", 1).await;

        mock_post_with_auth(
            &server,
            "/job/demo/buildWithParameters",
            Some("token-1"),
            None,
            ResponseTemplate::new(201)
                .append_header("Location", "/queue/item/1/")
                .set_body_string("one"),
            1,
        )
        .await;

        mock_post_with_auth(
            &server,
            "/job/demo/buildWithParameters",
            Some("token-2"),
            None,
            ResponseTemplate::new(201)
                .append_header("Location", "/queue/item/2/")
                .set_body_string("two"),
            1,
        )
        .await;

        let client = build_async_authed_client(server.uri(), Duration::ZERO)?;

        let first = client
            .execute(jenkins_sdk::api::jobs::build_with_parameters(
                "demo",
                [("foo", "bar")],
            )?)
            .await?;
        assert_eq!(first.queue_item_id, 1);

        let second = client
            .execute(jenkins_sdk::api::jobs::build_with_parameters(
                "demo",
                [("foo", "baz")],
            )?)
            .await?;
        assert_eq!(second.queue_item_id, 2);

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_client_propagates_http_error() -> Result<()> {
        let server = MockServer::start().await;

        mock_get(
            &server,
            "/queue/api/json",
            ResponseTemplate::new(404).set_body_json(json!({
                "message": "queue not found"
            })),
            1,
        )
        .await;

        let client = build_async_client(server.uri())?;

        let err = client
            .execute(jenkins_sdk::api::queue::list())
            .await
            .expect_err("expected HTTP error");

        match err {
            Error::Http(http) => {
                assert_eq!(http.status, StatusCode::NOT_FOUND);
                assert!(
                    http.body_snippet
                        .as_deref()
                        .unwrap_or_default()
                        .contains("queue not found")
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_client_attaches_crumb_and_basic_auth() -> Result<()> {
        let server = MockServer::start().await;

        mock_crumb(&server, "token", 1).await;

        mock_post_with_auth(
            &server,
            "/job/demo/buildWithParameters",
            Some("token"),
            Some("foo=bar"),
            ResponseTemplate::new(201).append_header("Location", "/queue/item/1/"),
            1,
        )
        .await;

        let client = build_async_authed_client(server.uri(), Duration::from_secs(300))?;

        client
            .execute(jenkins_sdk::api::jobs::build_with_parameters(
                "demo",
                [("foo", "bar")],
            )?)
            .await?;

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_client_supports_base_path_with_crumb() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/jenkins/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "crumbRequestField": "Jenkins-Crumb",
                "crumb": "token"
            })))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        mock_post_with_auth(
            &server,
            "/jenkins/job/demo/buildWithParameters",
            Some("token"),
            Some("foo=bar"),
            ResponseTemplate::new(201).append_header("Location", "/jenkins/queue/item/3/"),
            1,
        )
        .await;

        let base_url = format!("{}/jenkins", server.uri());
        let client = build_async_authed_client(base_url, Duration::from_secs(300))?;

        let triggered = client
            .execute(jenkins_sdk::api::jobs::build_with_parameters(
                "demo",
                [("foo", "bar")],
            )?)
            .await?;
        assert_eq!(triggered.queue_item_id, 3);

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_retry_replays_on_server_errors() -> Result<()> {
        let server = MockServer::start().await;

        mock_get(&server, "/queue/api/json", ResponseTemplate::new(503), 1).await;
        mock_get(
            &server,
            "/queue/api/json",
            ResponseTemplate::new(200).set_body_json(json!({ "items": [] })),
            1,
        )
        .await;

        let client = Client::builder(server.uri())?
            .configure_http(|http| {
                http.retry_policy(
                    jenkins_sdk::reqx::prelude::RetryPolicy::standard()
                        .max_attempts(4)
                        .base_backoff(Duration::from_millis(5)),
                )
            })
            .build()?;

        let response = client.execute(jenkins_sdk::api::queue::list()).await?;
        assert!(response.items.is_empty());

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_retry_replays_on_rate_limited_with_retry_after() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/queue/api/json"))
            .respond_with(ResponseTemplate::new(429).append_header("Retry-After", "0"))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        mock_get(
            &server,
            "/queue/api/json",
            ResponseTemplate::new(200).set_body_json(json!({ "items": [] })),
            1,
        )
        .await;

        let client = Client::builder(server.uri())?
            .configure_http(|http| {
                http.retry_policy(
                    jenkins_sdk::reqx::prelude::RetryPolicy::standard()
                        .max_attempts(2)
                        .base_backoff(Duration::from_millis(1))
                        .max_backoff(Duration::from_millis(1))
                        .jitter_ratio(0.0),
                )
            })
            .build()?;

        let response = client.execute(jenkins_sdk::api::queue::list()).await?;
        assert!(response.items.is_empty());

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_error_body_snippet_redacts_auth_token() -> Result<()> {
        let server = MockServer::start().await;

        mock_get(
            &server,
            "/queue/api/json",
            ResponseTemplate::new(500).set_body_string("supersecret"),
            1,
        )
        .await;

        let client = Client::builder(server.uri())?
            .auth(jenkins_sdk::Auth::basic(TEST_USER, "supersecret"))
            .build()?;

        let err = client
            .execute(jenkins_sdk::api::queue::list())
            .await
            .expect_err("expected HTTP error");

        match err {
            Error::Http(http) => {
                let snippet = http.body_snippet.as_deref().unwrap_or_default();
                assert!(!snippet.contains("supersecret"));
                assert!(snippet.contains("<redacted>"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        server.verify().await;
        Ok(())
    }
}

#[cfg(feature = "async")]
mod async_jobs {
    use anyhow::Result;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    use crate::fixture::{build_async_client, mock_get};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_jobs_info_uses_tree_query_param() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/json"))
            .and(query_param("tree", "jobs[name,url,color]"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jobs": []
            })))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = build_async_client(server.uri())?;

        let jobs = client.execute(jenkins_sdk::api::jobs::list()).await?;
        assert!(jobs.jobs.is_empty());

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_jobs_progressive_console_text_parses_headers() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/job/demo/1/logText/progressiveText"))
            .and(query_param("start", "0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("X-Text-Size", "5")
                    .append_header("X-More-Data", "true")
                    .set_body_string("hello"),
            )
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let client = build_async_client(server.uri())?;

        let chunk = client
            .execute(jenkins_sdk::api::builds::progressive_console_text(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
                0,
            )?)
            .await?;
        assert_eq!(chunk.text, "hello");
        assert_eq!(chunk.next_start, 5);
        assert!(chunk.more_data);

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_jobs_download_artifact_splits_path_segments() -> Result<()> {
        let server = MockServer::start().await;

        mock_get(
            &server,
            "/job/demo/1/artifact/a/b%20c.txt",
            ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]),
            1,
        )
        .await;

        let client = build_async_client(server.uri())?;

        let bytes = client
            .execute(jenkins_sdk::api::builds::download_artifact(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
                "a/b c.txt",
            )?)
            .await?;
        assert_eq!(bytes, vec![1, 2, 3]);

        server.verify().await;
        Ok(())
    }
}

#[cfg(feature = "async")]
mod async_system {
    use anyhow::Result;
    use wiremock::{MockServer, ResponseTemplate};

    use crate::fixture::{build_async_client, mock_get};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn async_system_downloads_jnlp_jars() -> Result<()> {
        let server = MockServer::start().await;

        mock_get(
            &server,
            "/jnlpJars/agent.jar",
            ResponseTemplate::new(200).set_body_bytes(vec![1]),
            1,
        )
        .await;
        mock_get(
            &server,
            "/jnlpJars/jenkins-cli.jar",
            ResponseTemplate::new(200).set_body_bytes(vec![2]),
            1,
        )
        .await;

        let client = build_async_client(server.uri())?;

        assert_eq!(
            client
                .execute(jenkins_sdk::api::system::agent_jar())
                .await?,
            vec![1]
        );
        assert_eq!(
            client.execute(jenkins_sdk::api::system::cli_jar()).await?,
            vec![2]
        );

        server.verify().await;
        Ok(())
    }
}

#[cfg(feature = "blocking")]
mod blocking_core {
    use std::time::Duration;

    use anyhow::Result;
    use jenkins_sdk::BlockingClient;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use crate::fixture::{
        TEST_TOKEN, TEST_USER, build_blocking_authed_client, mock_crumb, mock_post_with_auth,
        run_blocking,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_client_reuses_crumb_with_retry() -> Result<()> {
        let server = MockServer::start().await;

        mock_crumb(&server, "token", 1).await;

        mock_post_with_auth(
            &server,
            "/job/demo/1/stop",
            Some("token"),
            None,
            ResponseTemplate::new(503),
            1,
        )
        .await;

        mock_post_with_auth(
            &server,
            "/job/demo/1/stop",
            Some("token"),
            None,
            ResponseTemplate::new(200).set_body_string("stopped"),
            1,
        )
        .await;

        let base_url = server.uri();
        run_blocking(move || -> Result<()> {
            let client = BlockingClient::builder(base_url)?
                .auth(jenkins_sdk::Auth::basic(TEST_USER, TEST_TOKEN))
                .configure_http(|http| {
                    http.retry_policy(
                        jenkins_sdk::reqx::prelude::RetryPolicy::standard()
                            .max_attempts(4)
                            .base_backoff(Duration::from_millis(5))
                            .max_backoff(Duration::from_millis(5))
                            .jitter_ratio(0.0),
                    )
                })
                .configure_http(|http| {
                    http.allow_non_idempotent_retries(true)
                        .retry_eligibility(std::sync::Arc::new(
                            jenkins_sdk::reqx::advanced::PermissiveRetryEligibility,
                        ))
                })
                .crumb_ttl(Duration::from_secs(300))
                .build()?;

            client.execute(jenkins_sdk::api::builds::stop(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
            )?)?;
            Ok(())
        })
        .await?;

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_client_supports_base_path_with_crumb() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/jenkins/crumbIssuer/api/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "crumbRequestField": "Jenkins-Crumb",
                "crumb": "token"
            })))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        mock_post_with_auth(
            &server,
            "/jenkins/job/demo/1/stop",
            Some("token"),
            None,
            ResponseTemplate::new(200).set_body_string("stopped"),
            1,
        )
        .await;

        let base_url = format!("{}/jenkins", server.uri());
        run_blocking(move || -> Result<()> {
            let client = build_blocking_authed_client(base_url, Duration::from_secs(300))?;
            client.execute(jenkins_sdk::api::builds::stop(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
            )?)?;
            Ok(())
        })
        .await?;

        server.verify().await;
        Ok(())
    }
}

#[cfg(feature = "blocking")]
mod blocking_jobs {
    use anyhow::Result;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param},
    };

    use crate::fixture::{build_blocking_client, run_blocking};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_jobs_progressive_console_text_parses_headers() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/job/demo/1/logText/progressiveText"))
            .and(query_param("start", "0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("X-Text-Size", "5")
                    .append_header("X-More-Data", "true")
                    .set_body_string("hello"),
            )
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let base_url = server.uri();
        run_blocking(move || -> Result<()> {
            let client = build_blocking_client(base_url)?;

            let chunk = client.execute(jenkins_sdk::api::builds::progressive_console_text(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
                0,
            )?)?;
            assert_eq!(chunk.text, "hello");
            assert_eq!(chunk.next_start, 5);
            assert!(chunk.more_data);

            Ok(())
        })
        .await?;

        server.verify().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_jobs_download_artifact_splits_path_segments() -> Result<()> {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/job/demo/1/artifact/a/b%20c.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1, 2, 3]))
            .expect(1)
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let base_url = server.uri();
        run_blocking(move || -> Result<()> {
            let client = build_blocking_client(base_url)?;

            let bytes = client.execute(jenkins_sdk::api::builds::download_artifact(
                "demo",
                jenkins_sdk::api::builds::BuildSelector::Number(1),
                "a/b c.txt",
            )?)?;
            assert_eq!(bytes, vec![1, 2, 3]);

            Ok(())
        })
        .await?;

        server.verify().await;
        Ok(())
    }
}
