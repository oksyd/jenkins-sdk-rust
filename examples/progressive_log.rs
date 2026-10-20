//! Follow a running build. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN,
//! JENKINS_JOB and JENKINS_BUILD (a number or Jenkins build selector).
use jenkins_sdk::{Auth, Client, api::builds};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .build()?;
    let job = std::env::var("JENKINS_JOB")?;
    let build = std::env::var("JENKINS_BUILD")?.parse()?;
    let mut start = 0;
    loop {
        let chunk = client
            .execute(builds::progressive_console_text(&job, build, start)?)
            .await?;
        print!("{}", chunk.text);
        if !chunk.more_data {
            break;
        }
        start = chunk.next_start;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}
