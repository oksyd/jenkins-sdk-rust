//! Trust a private CA. Set JENKINS_URL, JENKINS_USER, JENKINS_TOKEN and JENKINS_CA_PEM.
use jenkins_sdk::{Auth, Client, api::queue, reqx::TlsRootStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ca = tokio::fs::read(std::env::var("JENKINS_CA_PEM")?).await?;
    let client = Client::builder(std::env::var("JENKINS_URL")?)?
        .auth(Auth::basic(
            std::env::var("JENKINS_USER")?,
            std::env::var("JENKINS_TOKEN")?,
        ))
        .configure_http(|http| {
            http.tls_root_store(TlsRootStore::Specific)
                .tls_root_ca_pem(ca)
        })
        .build()?;
    client.execute(queue::list()).await?;
    Ok(())
}
