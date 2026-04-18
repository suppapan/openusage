use clap::Parser;
use include_dir::{Dir, include_dir};
use openusage_plugin_engine::{manifest, runtime};
use openusage_shared::{MachinePush, PluginSnapshot};
use std::path::PathBuf;

const DEFAULT_LOCAL_API: &str = "http://127.0.0.1:6736";

// Bundle the entire plugins/ directory into the binary at compile time.
// At runtime we extract this into the agent's data dir and load from there.
static BUNDLED_PLUGINS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../plugins");

#[derive(Parser)]
#[command(
    name = "openusage-agent",
    about = "Headless agent that probes AI usage providers and pushes to a relay"
)]
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

    /// Source mode: "probe" runs plugins directly (default), "local-api" reads
    /// from a running OpenUsage desktop app's HTTP API, "cache-file" reads from
    /// a usage-api-cache.json file.
    #[arg(long, default_value = "probe")]
    source: String,

    /// Local OpenUsage API URL (only used when --source=local-api)
    #[arg(long)]
    local_api: Option<String>,

    /// Path to usage-api-cache.json (only used when --source=cache-file)
    #[arg(long)]
    cache_file: Option<PathBuf>,

    /// Plugin directory (only used in probe mode; defaults to bundled plugins
    /// extracted to ~/.openusage-agent/plugins)
    #[arg(long)]
    plugins_dir: Option<PathBuf>,

    /// Data directory for plugin state (defaults to ~/.openusage-agent)
    #[arg(long)]
    data_dir: Option<PathBuf>,
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

/// Extract bundled plugins to the given directory if not already present.
fn extract_bundled_plugins(target: &std::path::Path) -> Result<(), String> {
    if !target.exists() {
        std::fs::create_dir_all(target).map_err(|e| format!("create plugins dir: {}", e))?;
    }
    BUNDLED_PLUGINS
        .extract(target)
        .map_err(|e| format!("extract plugins: {}", e))?;
    Ok(())
}

/// Convert engine PluginOutput to wire PluginSnapshot via JSON round-trip.
/// Both have the same JSON shape; this avoids an explicit conversion impl.
fn to_snapshot(
    output: &runtime::PluginOutput,
    fetched_at: &str,
) -> Result<PluginSnapshot, String> {
    let mut value = serde_json::to_value(output).map_err(|e| format!("serialize: {}", e))?;
    if let Some(obj) = value.as_object_mut() {
        // Drop icon_url (snapshots don't include it; dashboard already has icons)
        obj.remove("iconUrl");
        obj.insert(
            "fetchedAt".to_string(),
            serde_json::Value::String(fetched_at.to_string()),
        );
    }
    serde_json::from_value(value).map_err(|e| format!("deserialize as snapshot: {}", e))
}

fn collect_via_probe(
    plugins_dir: &std::path::Path,
    data_dir: &PathBuf,
) -> Result<Vec<PluginSnapshot>, String> {
    let plugins = manifest::load_plugins_from_dir(plugins_dir);
    if plugins.is_empty() {
        return Err(format!("no plugins found in {}", plugins_dir.display()));
    }
    log::info!("probing {} plugins", plugins.len());
    let mut snapshots = Vec::new();
    let agent_version = env!("CARGO_PKG_VERSION");
    for plugin in &plugins {
        let output = runtime::run_probe(plugin, data_dir, agent_version);
        // Skip plugins that returned only an error badge
        let is_error = output.lines.len() == 1
            && matches!(
                &output.lines[0],
                runtime::MetricLine::Badge { label, .. } if label == "Error"
            );
        if is_error {
            log::warn!("skip {} (error result)", output.provider_id);
            continue;
        }
        match to_snapshot(&output, &now_rfc3339()) {
            Ok(snap) => {
                log::info!("ok {} ({} lines)", snap.provider_id, snap.lines.len());
                snapshots.push(snap);
            }
            Err(e) => log::warn!("convert {} failed: {}", output.provider_id, e),
        }
    }
    Ok(snapshots)
}

async fn collect_via_local_api(
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
        return Err(format!("local API returned {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("read response: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("parse JSON: {}", e))
}

fn collect_via_cache_file(path: &PathBuf) -> Result<Vec<PluginSnapshot>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read cache file: {}", e))?;
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
            Err(e) => log::warn!("skip malformed snapshot: {}", e),
        }
    }
    Ok(snapshots)
}

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
        .map_err(|e| format!("relay request failed: {}", e))?;
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
        .clone()
        .unwrap_or_else(|| machine_id.clone());

    // Resolve data dir for probe mode (also where we extract bundled plugins)
    let data_dir = cli.data_dir.clone().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".openusage-agent")
    });
    let plugins_dir = cli.plugins_dir.clone().unwrap_or_else(|| data_dir.join("plugins"));

    log::info!(
        "openusage-agent starting: machine={}, relay={}, source={}, interval={}s",
        machine_name, cli.relay, cli.source, cli.interval
    );

    // Extract bundled plugins on first run (probe mode only)
    if cli.source == "probe" && cli.plugins_dir.is_none() {
        if let Err(e) = extract_bundled_plugins(&plugins_dir) {
            log::error!("failed to extract bundled plugins: {}", e);
            std::process::exit(1);
        }
        log::info!("plugins extracted to {}", plugins_dir.display());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("create HTTP client");

    let mut tick = tokio::time::interval(std::time::Duration::from_secs(cli.interval));

    loop {
        tick.tick().await;

        let snapshots_result = match cli.source.as_str() {
            "probe" => {
                let dir = plugins_dir.clone();
                let data = data_dir.clone();
                tokio::task::spawn_blocking(move || collect_via_probe(&dir, &data))
                    .await
                    .unwrap_or_else(|e| Err(format!("probe task panicked: {}", e)))
            }
            "local-api" => {
                let api = cli
                    .local_api
                    .clone()
                    .unwrap_or_else(|| DEFAULT_LOCAL_API.to_string());
                collect_via_local_api(&client, &api).await
            }
            "cache-file" => {
                let path = match &cli.cache_file {
                    Some(p) => p.clone(),
                    None => {
                        log::error!("--cache-file required when --source=cache-file");
                        continue;
                    }
                };
                tokio::task::spawn_blocking(move || collect_via_cache_file(&path))
                    .await
                    .unwrap_or_else(|e| Err(format!("read task panicked: {}", e)))
            }
            other => Err(format!("unknown source mode: {}", other)),
        };

        let snapshots = match snapshots_result {
            Ok(s) => s,
            Err(e) => {
                log::error!("collect failed: {}", e);
                continue;
            }
        };

        log::info!("collected {} snapshots", snapshots.len());
        if snapshots.is_empty() {
            log::debug!("nothing to push");
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
            Ok(()) => log::info!("pushed to relay"),
            Err(e) => log::error!("push failed: {}", e),
        }
    }
}
