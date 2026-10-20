//! Read job and queue data. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN, JENKINS_JOB.
use jenkins_sdk::{
    Auth, Client,
    api::{builds, queue},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .build()?;
    let queue = client.execute(queue::list().tree("items[id,why]")).await?;
    println!("queued items: {}", queue.items.len());
    let job = std::env::var("JENKINS_JOB")?;
    let build = client
        .execute(builds::get(&job, builds::BuildSelector::LastSuccessful)?)
        .await?;
    println!("last successful build: {build:?}");
    Ok(())
}
