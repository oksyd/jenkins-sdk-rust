//! Blocking example. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN.
use jenkins_sdk::{Auth, BlockingClient, api::queue};

fn main() -> anyhow::Result<()> {
    let client = BlockingClient::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .build()?;
    let queue = client.execute(queue::list().tree("items[id,why]"))?;
    println!("queued items: {}", queue.items.len());
    Ok(())
}
