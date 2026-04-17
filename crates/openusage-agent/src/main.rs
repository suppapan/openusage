use clap::Parser;
use openusage_shared::{MachinePush, PluginSnapshot};
use std::path::PathBuf;

const DEFAULT_LOCAL_API: &str = "http://127.0.0.1:6736";

#[derive(Parser)]
#[command(name = "openusage-agent", about = "Headless agent that forwards OpenUsage data to a relay server")]
struct Cli {
    /// Sync token (generated in the dashboard app)
    #[arg(long)]
    token: String,

    /// Relay server URL (e.g. https://relay.example.com:8090)
    #[arg(long)]
    relay: String,

    /// Machine display name (defaults to hostname)
    #[arg(long)]
    machine_name: Option<String>,

    /// Probe/push interval in seconds (default: 300 = 5 min)
    #[arg(long, default_value_t = 300)]
    interval: u64,

    /// Local OpenUsage API URL to read from (default: http://127.0.0.1:6736)
    #[arg(long)]
    local_api: Option<String>,

    /// Path to usage-api-cache.json file (alternative to local API)
    #[arg(long)]
    cache_file: Option<PathBuf>,
}

fn now_rfc3339() -> String {
    let now = time::OffsetDateTime::now_utc();
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

fn resolve_machine_id() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown-machine".to_string())
}

/// Read snapshots from the local OpenUsage HTTP API.
async fn fetch_from_local_api(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<Vec<PluginSnapshot>, String> {
    let url = format!("{}/v1/usage", api_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("failed to reach local API at {}: {}", url, e))?;

    if !resp.status().is_success() {
        return Err(format!("local API returned status {}", resp.status()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("failed to read local API response: {}", e))?;

    serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse local API JSON: {}", e))
}

/// Read snapshots from the cache file directly.
fn read_from_cache_file(path: &PathBuf) -> Result<Vec<PluginSnapshot>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read cache file: {}", e))?;

    let file: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("invalid cache JSON: {}", e))?;

    let snapshots_obj = file
        .get("snapshots")
        .and_then(|v| v.as_object())
        .ok_or("cache file missing 'snapshots' object")?;

    let mut snapshots = Vec::new();
    for (_id, value) in snapshots_obj {
        match serde_json::from_value::<PluginSnapshot>(value.clone()) {
            Ok(s) => snapshots.push(s),
            Err(e) => log::warn!("skipping malformed snapshot: {}", e),
        }
    }

    Ok(snapshots)
}

/// Push snapshots to the relay server.
async fn push_to_relay(
    client: &reqwest::Client,
    relay_url: &str,
    token: &str,
    push: &MachinePush,
) -> Result<(), String> {
    let url = format!("{}/v1/push", relay_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(push)
        .send()
        .await
        .map_err(|e| format!("failed to reach relay: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("relay returned {}: {}", status, body));
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let machine_id = resolve_machine_id();
    let machine_name = cli
        .machine_name
        .unwrap_or_else(|| machine_id.clone());
    let local_api = cli
        .local_api
        .unwrap_or_else(|| DEFAULT_LOCAL_API.to_string());

    log::info!(
        "openusage-agent starting: machine={}, relay={}, interval={}s",
        machine_name,
        cli.relay,
        cli.interval
    );

    if cli.cache_file.is_some() {
        log::info!("source: cache file {:?}", cli.cache_file.as_ref().unwrap());
    } else {
        log::info!("source: local API {}", local_api);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to create HTTP client");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(cli.interval));

    loop {
        interval.tick().await;

        // Collect snapshots
        let snapshots = if let Some(ref cache_path) = cli.cache_file {
            match read_from_cache_file(cache_path) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("read failed: {}", e);
                    continue;
                }
            }
        } else {
            match fetch_from_local_api(&client, &local_api).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("fetch failed: {}", e);
                    continue;
                }
            }
        };

        log::info!("collected {} snapshots", snapshots.len());

        if snapshots.is_empty() {
            log::debug!("no snapshots to push, skipping");
            continue;
        }

        let push = MachinePush {
            machine_id: machine_id.clone(),
            machine_name: machine_name.clone(),
            snapshots,
            pushed_at: now_rfc3339(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        match push_to_relay(&client, &cli.relay, &cli.token, &push).await {
            Ok(()) => log::info!("pushed to relay successfully"),
            Err(e) => log::error!("push failed: {}", e),
        }
    }
}
