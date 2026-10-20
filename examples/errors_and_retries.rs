//! Native retry policy and SDK errors. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN.
use jenkins_sdk::{Auth, Client, Error, api::queue, reqx::prelude::RetryPolicy};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .configure_http(|http| {
            http.retry_policy(RetryPolicy::standard().max_attempts(3))
                .total_timeout(Some(Duration::from_secs(20)))
        })
        .build()?;
    match client.execute(queue::list()).await {
        Ok(queue) => println!("queued items: {}", queue.items.len()),
        Err(Error::Http(response)) => {
            eprintln!(
                "{response}, request_id={:?}, retry_after={:?}",
                response.request_id, response.retry_after
            );
        }
        Err(Error::Transport(source)) => eprintln!("HTTP transport: {source}"),
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
