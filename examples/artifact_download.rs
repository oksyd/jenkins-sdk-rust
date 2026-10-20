//! Download an artifact from the last successful build. Set JENKINS_URL,
//! JENKINS_USER, JENKINS_TOKEN, JENKINS_JOB, JENKINS_ARTIFACT and OUTPUT_PATH.
use jenkins_sdk::{Auth, Client, api::builds};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .configure_http(|http| http.max_response_body_bytes(64 * 1024 * 1024))
        .build()?;
    let job = std::env::var("JENKINS_JOB")?;
    let artifact = std::env::var("JENKINS_ARTIFACT")?;
    let bytes = client
        .execute(builds::download_artifact(
            &job,
            builds::BuildSelector::LastSuccessful,
            &artifact,
        )?)
        .await?;
    tokio::fs::write(std::env::var("OUTPUT_PATH")?, &bytes).await?;
    println!("downloaded {} bytes", bytes.len());
    Ok(())
}
