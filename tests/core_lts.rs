//! Real, plugin-free Jenkins contracts. Run with tests/jenkins-lts.sh.
//! Each enabled transport runs the same lifecycle against an isolated controller.
use anyhow::{Context, Result, ensure};
use jenkins_sdk::{
    Auth, Builder, Multipart, Request, SUPPORTED_JENKINS_LTS,
    api::{
        builds::{self, BuildSelector},
        computers, executors, fingerprints, jobs, labels, logs, monitors, plugins, queue, remote,
        setup, system, tokens, updates, users, views,
    },
};
use serde_json::{Value, json};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
enum Client {
    #[cfg(feature = "async")]
    Async(jenkins_sdk::Client),
    #[cfg(feature = "blocking")]
    Blocking(jenkins_sdk::BlockingClient),
}

fn configure<T>(builder: Builder<T>) -> Builder<T> {
    builder
        .auth(Auth::basic("sdk-admin", "sdk-integration-password"))
        .crumb_ttl(Duration::from_secs(60))
        .configure_http(|http| {
            http.total_timeout(Some(Duration::from_secs(30)))
                .max_response_body_bytes(32 * 1024 * 1024)
        })
}

impl Client {
    fn with_token(&self, token: &str) -> Result<Self> {
        Ok(match self {
            #[cfg(feature = "async")]
            Self::Async(client) => Self::Async(
                jenkins_sdk::Client::builder(client.base_url())?
                    .auth(Auth::basic("sdk-admin", token))
                    .build()?,
            ),
            #[cfg(feature = "blocking")]
            Self::Blocking(client) => Self::Blocking(
                jenkins_sdk::BlockingClient::builder(client.base_url())?
                    .auth(Auth::basic("sdk-admin", token))
                    .build()?,
            ),
        })
    }

    async fn execute<T: Send + 'static>(&self, request: Request<T>) -> Result<T> {
        let context = format!("{request:?}");
        let result = match self {
            #[cfg(feature = "async")]
            Self::Async(client) => client.execute(request).await,
            #[cfg(feature = "blocking")]
            Self::Blocking(client) => {
                let client = client.clone();
                tokio::task::spawn_blocking(move || client.execute(request)).await?
            }
        };
        result.with_context(|| context)
    }

    async fn finished_build(&self, job: &str, id: u64) -> Result<u64> {
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let item = self.execute(queue::item(id)).await?;
            if let Some(number) = item.executable.and_then(|build| build.number) {
                loop {
                    let build = self
                        .execute(builds::get(job, BuildSelector::Number(number))?)
                        .await?;
                    if build.building == Some(false) {
                        ensure!(
                            build.result.as_deref() == Some("SUCCESS"),
                            "build failed: {build:?}"
                        );
                        return Ok(number);
                    }
                    ensure!(Instant::now() < deadline, "build did not complete");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
            ensure!(Instant::now() < deadline, "build stayed in queue");
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the isolated Jenkins fixture; run tests/jenkins-lts.sh"]
async fn core_lts_contracts() -> Result<()> {
    let base = std::env::var("JENKINS_SDK_TEST_URL").context("missing fixture URL")?;
    let clients = [
        #[cfg(feature = "async")]
        Client::Async(configure(jenkins_sdk::Client::builder(&base)?).build()?),
        #[cfg(feature = "blocking")]
        Client::Blocking(configure(jenkins_sdk::BlockingClient::builder(&base)?).build()?),
    ];
    for (mode, client) in clients.iter().enumerate() {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let prefix = format!("sdk-{mode}-{suffix}");
        eprintln!("Running core LTS lifecycle for transport {mode}");
        read_core(client).await?;
        jobs_and_builds(client, &prefix).await?;
        nodes_and_labels(client, &prefix).await?;
        views_and_users(client, &prefix).await?;
        management(client, &prefix).await?;
    }
    plugin_management(&clients[0]).await?;
    Ok(())
}

async fn read_core(client: &Client) -> Result<()> {
    assert_eq!(
        client.execute(system::version()).await?,
        SUPPORTED_JENKINS_LTS
    );
    assert!(client.execute(plugins::list()).await?.plugins.is_empty());
    assert_eq!(client.execute(system::health()).await?["status"], true);
    assert_eq!(
        client.execute(system::who_am_i()).await?.name.as_deref(),
        Some("sdk-admin")
    );
    client.execute(system::root()).await?;
    client.execute(system::crumb()).await?;
    client.execute(jobs::list()).await?;
    client.execute(views::list()).await?;
    client.execute(system::overall_load()).await?;
    client.execute(system::unlabeled_load()).await?;
    assert_eq!(
        client
            .execute(system::script("print Jenkins.VERSION"))
            .await?,
        SUPPORTED_JENKINS_LTS
    );
    let xml = client
        .execute(
            remote::xml("")?
                .query("tree", "mode,numExecutors")
                .append_query("exclude", "/*/mode")
                .append_query("exclude", "/*/numExecutors"),
        )
        .await?;
    let xml = String::from_utf8(xml)?;
    assert!(!xml.contains("<mode>") && !xml.contains("<numExecutors>"));
    assert!(!client.execute(remote::schema("")?).await?.is_empty());
    assert!(
        !client
            .execute(remote::python("")?.tree("mode"))
            .await?
            .is_empty()
    );
    let root: Value = client
        .execute(remote::json("")?.tree("numExecutors"))
        .await?;
    assert_eq!(root["numExecutors"], 2);
    assert!(
        client
            .execute(system::agent_jar())
            .await?
            .starts_with(b"PK")
    );
    assert!(client.execute(system::cli_jar()).await?.starts_with(b"PK"));
    Ok(())
}

async fn jobs_and_builds(client: &Client, prefix: &str) -> Result<()> {
    let job = format!("{prefix} job");
    let xml = r#"<project>
      <description>initial</description><keepDependencies>false</keepDependencies>
      <properties/><scm class="hudson.scm.NullSCM"/><canRoam>true</canRoam>
      <disabled>false</disabled><quietPeriod>0</quietPeriod><triggers><hudson.triggers.SCMTrigger><spec/></hudson.triggers.SCMTrigger></triggers>
      <builders><hudson.tasks.Shell><command>printf 'hello sdk' &gt; result.txt</command></hudson.tasks.Shell></builders>
      <publishers><hudson.tasks.ArtifactArchiver><artifacts>result.txt</artifacts><fingerprint>true</fingerprint></hudson.tasks.ArtifactArchiver></publishers>
      <buildWrappers/>
    </project>"#;
    client.execute(jobs::create_from_xml(&job, xml)?).await?;
    assert_eq!(
        client.execute(jobs::get(&job)?).await?.name.as_deref(),
        Some(job.as_str())
    );
    client
        .execute(jobs::set_description(&job, "updated")?)
        .await?;
    assert_eq!(client.execute(jobs::description(&job)?).await?, "updated");
    let config = client.execute(jobs::get_config_xml(&job)?).await?;
    client
        .execute(jobs::update_config_xml(&job, config)?)
        .await?;
    client.execute(jobs::reload(&job)?).await?;
    client.execute(jobs::disable(&job)?).await?;
    assert_eq!(
        client.execute(jobs::get(&job)?).await?.buildable,
        Some(false)
    );
    client.execute(jobs::enable(&job)?).await?;
    client.execute(jobs::get(&job)?.tree("scm[*]")).await?;
    client.execute(jobs::poll(&job)?).await?;
    let trigger = client.execute(jobs::build(&job)?).await?;
    let number = client.finished_build(&job, trigger.queue_item_id).await?;
    let build = BuildSelector::Number(number);
    assert_eq!(
        client.execute(builds::number(&job, build)?).await?,
        number.to_string()
    );
    assert!(
        !client
            .execute(builds::timestamp(&job, build)?)
            .await?
            .is_empty()
    );
    assert!(
        client
            .execute(builds::console_text(&job, build)?)
            .await?
            .contains("SUCCESS")
    );
    let log = client
        .execute(builds::progressive_console_text(&job, build, 0)?)
        .await?;
    assert!(!log.more_data && log.next_start > 0);
    client
        .execute(builds::progressive_console_html(&job, build, 0)?)
        .await?;
    client.execute(jobs::polling_log(&job)?).await?;
    assert_eq!(
        client
            .execute(builds::download_artifact(&job, build, "result.txt")?)
            .await?,
        b"hello sdk"
    );
    assert_eq!(
        client
            .execute(jobs::workspace_file(&job, "result.txt")?)
            .await?,
        b"hello sdk"
    );
    assert!(
        client
            .execute(builds::artifact_archive(&job, build)?)
            .await?
            .starts_with(b"PK")
    );
    assert!(
        client
            .execute(jobs::workspace_archive(&job)?)
            .await?
            .starts_with(b"PK")
    );
    client
        .execute(builds::configure(
            &job,
            build,
            "SDK build",
            "SDK description",
        )?)
        .await?;
    assert_eq!(
        client
            .execute(builds::get(&job, build)?)
            .await?
            .display_name
            .as_deref(),
        Some("SDK build")
    );
    client
        .execute(builds::set_description(&job, build, "changed")?)
        .await?;
    client
        .execute(builds::toggle_keep_log(&job, build)?)
        .await?;
    client
        .execute(builds::toggle_keep_log(&job, build)?)
        .await?;
    let fingerprint = client
        .execute(fingerprints::check("result.txt", b"hello sdk".to_vec())?)
        .await?;
    let location = fingerprint
        .headers()
        .get("location")
        .context("fingerprint location")?
        .to_str()?;
    let hash = location
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .context("fingerprint hash")?;
    let data = client.execute(fingerprints::get(hash)?).await?;
    assert_eq!(data["fileName"], "result.txt");
    client.execute(jobs::wipe_workspace(&job)?).await?;
    let copy = format!("{prefix}-copy");
    client.execute(jobs::copy(&job, &copy)?).await?;
    let renamed = format!("{prefix}-renamed");
    client.execute(jobs::rename(&copy, &renamed)?).await?;
    client.execute(jobs::delete(&renamed)?).await?;
    client.execute(builds::delete(&job, build)?).await?;
    client.execute(jobs::delete(&job)?).await?;

    let parameter_job = format!("{prefix}-parameters");
    let xml = r#"<project><properties><hudson.model.ParametersDefinitionProperty><parameterDefinitions>
      <hudson.model.StringParameterDefinition><name>MESSAGE</name><defaultValue>default</defaultValue></hudson.model.StringParameterDefinition>
      <hudson.model.FileParameterDefinition><name>upload.bin</name></hudson.model.FileParameterDefinition>
      </parameterDefinitions></hudson.model.ParametersDefinitionProperty></properties>
      <scm class="hudson.scm.NullSCM"/><canRoam>true</canRoam><quietPeriod>0</quietPeriod>
      <builders><hudson.tasks.Shell><command>test "$MESSAGE" = 'from multipart' &amp;&amp; test "$(cat upload.bin)" = payload</command></hudson.tasks.Shell></builders></project>"#;
    client
        .execute(jobs::create_from_xml(&parameter_job, xml)?)
        .await?;
    let form = Multipart::new().text("MESSAGE", "from multipart")?.file(
        "upload.bin",
        "source.bin",
        b"payload".to_vec(),
    )?;
    let trigger = client
        .execute(jobs::build_with_files(&parameter_job, form)?)
        .await?;
    client
        .finished_build(&parameter_job, trigger.queue_item_id)
        .await?;
    // Delay keeps the item queued so cancellation can be checked deterministically.
    let trigger = client
        .execute(
            jobs::build_with_parameters(&parameter_job, [("MESSAGE", "cancel")])?
                .query("delay", "3600sec"),
        )
        .await?;
    client.execute(queue::cancel(trigger.queue_item_id)).await?;
    let trigger = client
        .execute(
            jobs::build_with_parameters(&parameter_job, [("MESSAGE", "cancel")])?
                .query("delay", "3600sec"),
        )
        .await?;
    client.execute(jobs::cancel_queue(&parameter_job)?).await?;
    assert!(
        client
            .execute(queue::list())
            .await?
            .items
            .iter()
            .all(|item| item.id != Some(trigger.queue_item_id))
    );
    client.execute(jobs::delete(&parameter_job)?).await?;
    let stopped_job = format!("{prefix}-stop");
    client.execute(jobs::create_from_xml(&stopped_job,
        "<project><quietPeriod>0</quietPeriod><builders><hudson.tasks.Shell><command>sleep 60</command></hudson.tasks.Shell></builders></project>")?).await?;
    let trigger = client.execute(jobs::build(&stopped_job)?).await?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let number = loop {
        if let Some(number) = client
            .execute(queue::item(trigger.queue_item_id))
            .await?
            .executable
            .and_then(|build| build.number)
        {
            break number;
        }
        ensure!(Instant::now() < deadline, "stop fixture did not start");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    client
        .execute(builds::stop(&stopped_job, BuildSelector::Number(number))?)
        .await?;
    loop {
        let build = client
            .execute(builds::get(&stopped_job, BuildSelector::Number(number))?)
            .await?;
        if build.building == Some(false) {
            assert_eq!(build.result.as_deref(), Some("ABORTED"));
            break;
        }
        ensure!(Instant::now() < deadline, "build did not stop");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    client.execute(jobs::delete(&stopped_job)?).await?;
    Ok(())
}

async fn nodes_and_labels(client: &Client, prefix: &str) -> Result<()> {
    let node = format!("{prefix}-agent");
    let xml = format!(
        r#"<slave><name>{node}</name><description>test</description>
      <remoteFS>/tmp/sdk-agent</remoteFS><numExecutors>1</numExecutors><mode>NORMAL</mode>
      <retentionStrategy class="hudson.slaves.RetentionStrategy$Always"/>
      <launcher class="hudson.slaves.JNLPLauncher"/><label>sdk-label</label><nodeProperties/></slave>"#
    );
    client
        .execute(computers::create_from_xml(&node, xml)?)
        .await?;
    assert_eq!(
        client.execute(computers::get(&node)?).await?.offline,
        Some(true)
    );
    client
        .execute(computers::set_description(&node, "updated")?)
        .await?;
    let xml = client.execute(computers::get_config_xml(&node)?).await?;
    client
        .execute(computers::update_config_xml(&node, xml)?)
        .await?;
    client
        .execute(computers::toggle_offline(&node, Some("maintenance"))?)
        .await?;
    client
        .execute(computers::change_offline_cause(&node, "new reason")?)
        .await?;
    client
        .execute(computers::toggle_offline(&node, None)?)
        .await?;
    client.execute(computers::disconnect(&node)?).await?;
    client.execute(computers::export_table(&node)?).await?;
    assert!(
        !client
            .execute(computers::agent_jnlp(&node)?)
            .await?
            .is_empty()
    );
    client
        .execute(computers::progressive_log(&node, 0)?)
        .await?;
    client.execute(computers::update_monitors()).await?;
    client.execute(computers::list()).await?;
    client.execute(computers::executors_info()).await?;
    client
        .execute(executors::get(
            "(built-in)",
            executors::Executor::Regular(0),
        )?)
        .await?;
    client
        .execute(executors::stop_build(
            "(built-in)",
            executors::Executor::Regular(0),
            "absent#1",
        )?)
        .await?;
    client
        .execute(executors::stop(
            "(built-in)",
            executors::Executor::Regular(0),
        )?)
        .await?;
    client.execute(labels::get("sdk-label")?).await?;
    client
        .execute(labels::load_statistics("sdk-label")?)
        .await?;
    client
        .execute(labels::set_description("sdk-label", "test label")?)
        .await?;
    client
        .execute(labels::configure(
            "sdk-label",
            &json!({"description": "configured"}),
        )?)
        .await?;
    assert_eq!(
        client.execute(labels::get("sdk-label")?).await?["description"],
        "configured"
    );
    let copy = format!("{prefix}-agent-copy");
    client.execute(computers::copy(&node, &copy)?).await?;
    client.execute(computers::delete(&copy)?).await?;
    client.execute(computers::delete(&node)?).await?;
    Ok(())
}

async fn views_and_users(client: &Client, prefix: &str) -> Result<()> {
    let view = format!("{prefix}-view");
    let xml = format!(
        "<listView><name>{view}</name><filterExecutors>false</filterExecutors><filterQueue>false</filterQueue><properties/><jobNames><comparator class=\"hudson.util.CaseInsensitiveComparator\"/></jobNames><jobFilters/><columns/></listView>"
    );
    client.execute(views::create_from_xml(&view, xml)?).await?;
    let job = format!("{prefix}-view-job");
    client
        .execute(jobs::create_from_xml(&job, "<project/>")?)
        .await?;
    client.execute(views::add_job(&view, &job)?).await?;
    assert!(
        client
            .execute(views::get(&view)?)
            .await?
            .jobs
            .context("view jobs")?
            .iter()
            .any(|item| item.name == job)
    );
    client.execute(views::remove_job(&view, &job)?).await?;
    client.execute(jobs::delete(&job)?).await?;
    client
        .execute(views::set_description(&view, "view description")?)
        .await?;
    let xml = client.execute(views::get_config_xml(&view)?).await?;
    client
        .execute(views::update_config_xml(&view, xml)?)
        .await?;
    let renamed = format!("{prefix}-new-view");
    client.execute(views::configure(&view, &json!({"name": renamed, "description": "renamed", "filterExecutors": false, "filterQueue": false}))?).await?;
    assert_eq!(
        client.execute(views::get(&renamed)?).await?.name.as_deref(),
        Some(renamed.as_str())
    );
    client
        .execute(remote::feed(&format!("view/{renamed}"), remote::Feed::All)?)
        .await?;
    client.execute(views::delete(&renamed)?).await?;
    let user = format!("{prefix}-user");
    client
        .execute(users::create_local(&user, "fixture-password", "SDK User")?)
        .await?;
    client
        .execute(users::set_description(&user, "user description")?)
        .await?;
    client
        .execute(users::configure(
            &user,
            &json!({"fullName": "Renamed User", "description": "account"}),
        )?)
        .await?;
    assert_eq!(
        client
            .execute(users::get(&user)?)
            .await?
            .full_name
            .as_deref(),
        Some("Renamed User")
    );
    client.execute(users::renew_session_seed(&user)?).await?;
    // Descriptor-owned resources share the public request path and CSRF handling.
    client.execute(Request::new(jenkins_sdk::http::Method::POST,
        ["user", &user, "my-views", "createView"])?
        .query("name", "personal")
        .xml("<listView><name>personal</name><jobNames><comparator class=\"hudson.util.CaseInsensitiveComparator\"/></jobNames><columns/><jobFilters/><properties/></listView>")
        .allow_action_redirects().discard_body()).await?;
    let personal = client
        .execute(
            Request::new(
                jenkins_sdk::http::Method::GET,
                ["user", &user, "my-views", "view", "personal", "api", "json"],
            )?
            .decode_json::<views::ViewInfo>(),
        )
        .await?;
    assert_eq!(personal.name.as_deref(), Some("personal"));
    client.execute(users::delete(&user)?).await?;
    let token = client
        .execute(tokens::generate(
            "sdk-admin",
            "temporary",
            tokens::Expiration::Days(7),
        )?)
        .await?;
    assert!(!format!("{token:?}").contains(&token.token_value));
    // API tokens authenticate mutations without a crumb or session cookie.
    let token_client = client.with_token(&token.token_value)?;
    token_client
        .execute(tokens::rename("sdk-admin", &token.token_uuid, "renamed")?)
        .await?;
    client
        .execute(tokens::revoke_all_except("sdk-admin", &token.token_uuid)?)
        .await?;
    client
        .execute(tokens::revoke("sdk-admin", &token.token_uuid)?)
        .await?;
    client
        .execute(tokens::add_fixed(
            "sdk-admin",
            "fixed",
            "110123456789abcdef0123456789abcdef",
            tokens::Expiration::Never,
        )?)
        .await?;
    client.execute(tokens::revoke_all("sdk-admin")?).await?;
    Ok(())
}

async fn management(client: &Client, prefix: &str) -> Result<()> {
    client.execute(plugins::available()).await?;
    client.execute(plugins::search("test", 5)).await?;
    assert!(
        client
            .execute(plugins::prevalidate_config("<project/>"))
            .await?
            .is_empty()
    );
    client
        .execute(plugins::install_required("<project/>"))
        .await?;
    client.execute(updates::get()).await?;
    client.execute(updates::installation_status()).await?;
    client.execute(updates::incomplete_installations()).await?;
    client.execute(updates::invalidate()).await?;
    client.execute(setup::restart_status()).await?;
    client.execute(monitors::upgrade_old_data("all")).await?;
    client.execute(monitors::discard_unreadable_data()).await?;
    client.execute(monitors::clear_slow_triggers()).await?;
    client.execute(logs::create(prefix)?).await?;
    client
        .execute(logs::configure(
            prefix,
            prefix,
            &[logs::Logger {
                name: "jenkins.model.Jenkins".into(),
                level: 800,
            }],
        )?)
        .await?;
    client
        .execute(logs::set_level("jenkins.model.Jenkins", "INFO"))
        .await?;
    client.execute(logs::feed(prefix)?).await?;
    client.execute(logs::system_feed()).await?;
    client.execute(logs::clear(prefix)?).await?;
    client.execute(logs::delete(prefix)?).await?;
    client
        .execute(system::quiet_down().query("reason", "integration test"))
        .await?;
    assert_eq!(
        client.execute(system::root()).await?.quieting_down,
        Some(true)
    );
    client.execute(system::cancel_quiet_down()).await?;
    client.execute(system::cleanup_fingerprints()).await?;
    client.execute(system::cleanup_workspaces()).await?;
    Ok(())
}

async fn plugin_management(client: &Client) -> Result<()> {
    // This JPI contains only a manifest, no code, dependencies, or contributed APIs.
    let archive = include_bytes!("fixtures/jenkins/sdk-fixture.jpi").to_vec();
    client
        .execute(plugins::upload("sdk-fixture.jpi", archive)?)
        .await?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if client
            .execute(plugins::list())
            .await?
            .plugins
            .iter()
            .any(|plugin| plugin.short_name == "sdk-fixture")
        {
            break;
        }
        ensure!(Instant::now() < deadline, "fixture plugin was not loaded");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        client
            .execute(plugins::get("sdk-fixture")?)
            .await?
            .version
            .as_deref(),
        Some("1.0")
    );
    client.execute(plugins::disable("sdk-fixture")?).await?;
    assert_eq!(
        client.execute(plugins::get("sdk-fixture")?).await?.enabled,
        Some(false)
    );
    client.execute(plugins::enable("sdk-fixture")?).await?;
    assert_eq!(
        client.execute(plugins::get("sdk-fixture")?).await?.enabled,
        Some(true)
    );
    client.execute(plugins::uninstall("sdk-fixture")?).await?;
    Ok(())
}
