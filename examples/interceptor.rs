//! Native reqx attempt interception. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN.
use jenkins_sdk::{
    Auth, Client,
    api::queue,
    reqx::advanced::{Interceptor, RequestContext},
};
use std::sync::Arc;

struct TraceAttempts;
impl Interceptor for TraceAttempts {
    fn on_request(&self, context: &RequestContext, headers: &mut http::HeaderMap) {
        println!(
            "{} {} attempt={}",
            context.method(),
            context.uri(),
            context.attempt()
        );
        headers.insert(
            "x-client",
            http::HeaderValue::from_static("jenkins-example"),
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .configure_http(|http| http.interceptor(Arc::new(TraceAttempts)))
        .build()?;
    client.execute(queue::list()).await?;
    Ok(())
}
