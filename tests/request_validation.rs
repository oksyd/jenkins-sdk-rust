use jenkins_sdk::{
    Error,
    api::{
        builds,
        computers::{self, ExecutorsInfo},
        jobs, users, views,
    },
};

#[test]
fn reject_ambiguous_paths_before_execution() {
    for path in [
        "", "/demo", "demo/", "a//b", ".", "..", "a/../b", "a\\b", "a\nb",
    ] {
        assert!(
            matches!(jobs::get(path), Err(Error::InvalidInput { .. })),
            "{path:?}"
        );
        assert!(builds::download_artifact("demo", builds::BuildSelector::Last, path).is_err());
    }
    for name in ["", ".", "..", "a/b", "a\\b", "a\nb"] {
        assert!(computers::get(name).is_err(), "{name:?}");
        assert!(users::get(name).is_err(), "{name:?}");
        assert!(views::get(name).is_err(), "{name:?}");
        assert!(
            jobs::create_from_xml(name, "<project/>").is_err(),
            "{name:?}"
        );
    }
    assert!(jobs::get("团队/release #1").is_ok());
    assert!(
        "lastSuccessfulBuild"
            .parse::<builds::BuildSelector>()
            .is_ok()
    );
    assert!("../config.xml".parse::<builds::BuildSelector>().is_err());
}

#[test]
fn reject_base_urls_with_implicit_credentials_or_request_state() {
    for url in [
        "ftp://jenkins.example.com",
        "https://user:secret@jenkins.example.com",
        "https://jenkins.example.com?token=secret",
        "https://jenkins.example.com#fragment",
        "/jenkins",
    ] {
        #[cfg(feature = "async")]
        assert!(jenkins_sdk::Client::builder(url).is_err(), "{url}");
        #[cfg(feature = "blocking")]
        assert!(jenkins_sdk::BlockingClient::builder(url).is_err(), "{url}");
    }
}

#[test]
fn executor_capacity_does_not_hide_inconsistent_server_data() {
    let valid: ExecutorsInfo =
        serde_json::from_str(r#"{"totalExecutors": 5, "busyExecutors": 3}"#).unwrap();
    assert_eq!(valid.idle_executors(), Some(2));
    let inconsistent: ExecutorsInfo =
        serde_json::from_str(r#"{"totalExecutors": 2, "busyExecutors": 3}"#).unwrap();
    assert_eq!(inconsistent.idle_executors(), None);
    assert!(serde_json::from_str::<ExecutorsInfo>(r#"{"totalExecutors": 2}"#).is_err());
}

#[test]
fn custom_paths_and_upload_headers_reject_injection() {
    use jenkins_sdk::{Multipart, Request, http::Method};
    for segments in [
        vec!["..", "config.xml"],
        vec!["", "api"],
        vec!["job", "a/b"],
        vec!["job", "a\\b"],
    ] {
        assert!(Request::new(Method::GET, segments).is_err());
    }
    assert!(Request::new(Method::GET, ["health", ""]).is_ok());
    assert!(Request::new(Method::GET, Vec::<String>::new()).is_ok());
    assert!(
        Multipart::new()
            .text("field\r\nInjected: value", "data")
            .is_err()
    );
    assert!(
        Multipart::new()
            .file("file", "name\n.txt", Vec::new())
            .is_err()
    );
    assert!(
        jenkins_sdk::api::tokens::add_fixed(
            "admin",
            "invalid",
            "bad",
            jenkins_sdk::api::tokens::Expiration::Never,
        )
        .is_err()
    );
}
