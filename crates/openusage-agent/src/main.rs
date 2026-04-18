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
    /// Sync token (generated in the dashboard app). Optional when --check is
    /// set and a saved config exists at ~/.openusage-agent/config.json.
    #[arg(long)]
    token: Option<String>,

    /// Relay server URL (e.g. https://relay.example.com:8090). Optional
    /// when --check is set and a saved config exists.
    #[arg(long)]
    relay: Option<String>,

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

    /// Run a one-shot diagnostic check (relay reachability, auth, per-plugin
    /// readiness) and exit. Does not start the loop.
    #[arg(long, default_value_t = false)]
    check: bool,
}

// ─── Persisted config (so --check can run without re-typing token/relay) ────

#[derive(serde::Serialize, serde::Deserialize)]
struct AgentConfig {
    token: String,
    relay: String,
}

fn config_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("config.json")
}

/// System-wide config path. Used as a fallback so non-root users can run
/// `openusage-agent --check` even if the service was installed by root.
fn system_config_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| "C:/ProgramData".into()))
            .join("OpenUsage")
            .join("agent.json")
    } else {
        PathBuf::from("/etc/openusage-agent/config.json")
    }
}

fn save_config(data_dir: &PathBuf, token: &str, relay: &str) {
    let _ = std::fs::create_dir_all(data_dir);
    let cfg = AgentConfig {
        token: token.to_string(),
        relay: relay.to_string(),
    };
    let json = match serde_json::to_string_pretty(&cfg) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = std::fs::write(config_path(data_dir), &json);
    // Also try writing the system path so non-root users can read it on
    // --check. Best-effort: silently ignore permission failures.
    let sys = system_config_path();
    if let Some(parent) = sys.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&sys, &json).is_ok() {
        // Make world-readable so unprivileged users can run --check.
        // (token is a shared sync secret, not a long-term credential.)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&sys, std::fs::Permissions::from_mode(0o644));
        }
    }
}

fn load_config(data_dir: &PathBuf) -> Option<AgentConfig> {
    // Prefer per-user config; fall back to system-wide.
    if let Ok(content) = std::fs::read_to_string(config_path(data_dir)) {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return Some(cfg);
        }
    }
    let content = std::fs::read_to_string(system_config_path()).ok()?;
    serde_json::from_str(&content).ok()
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

/// Plugin directory names to skip (test-only, not for production data).
const EXCLUDED_PLUGINS: &[&str] = &["mock"];

/// Extract bundled plugins to the given directory if not already present.
/// Skips test-only plugins (mock, etc.) so they don't appear in real dashboards.
fn extract_bundled_plugins(target: &std::path::Path) -> Result<(), String> {
    if !target.exists() {
        std::fs::create_dir_all(target).map_err(|e| format!("create plugins dir: {}", e))?;
    }
    for entry in BUNDLED_PLUGINS.entries() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if EXCLUDED_PLUGINS.contains(&name) {
            log::debug!("skipping excluded plugin '{}'", name);
            continue;
        }
        if let Some(_dir) = entry.as_dir() {
            // Ensure the per-plugin subdir exists so include_dir can write
            // its contents (e.g. plugin.json, plugin.js, icon.svg) into it.
            let plugin_subdir = target.join(name);
            std::fs::create_dir_all(&plugin_subdir)
                .map_err(|e| format!("mkdir {}: {}", plugin_subdir.display(), e))?;
            // Extract the directory's contents (target is the workspace root,
            // include_dir replays the relative paths under it).
            entry
                .as_dir()
                .unwrap()
                .extract(target)
                .map_err(|e| format!("extract {}: {}", name, e))?;
        } else if let Some(file) = entry.as_file() {
            log::debug!("skipping top-level file {:?}", file.path());
        }
    }
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
        if EXCLUDED_PLUGINS.contains(&plugin.manifest.id.as_str()) {
            log::debug!("skipping excluded plugin '{}'", plugin.manifest.id);
            continue;
        }
        let output = runtime::run_probe(plugin, data_dir, agent_version);
        // Push every result, even error ones. The dashboard renders the error
        // badge so the user can see exactly why a plugin isn't reporting (e.g.
        // 'Not logged in. Run `codex` to authenticate.') without SSHing to
        // the machine.
        let is_error = output.lines.len() == 1
            && matches!(
                &output.lines[0],
                runtime::MetricLine::Badge { label, .. } if label == "Error"
            );
        match to_snapshot(&output, &now_rfc3339()) {
            Ok(snap) => {
                if is_error {
                    log::info!("err {} (forwarding error to dashboard)", snap.provider_id);
                } else {
                    log::info!("ok {} ({} lines)", snap.provider_id, snap.lines.len());
                }
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

// ─── Diagnostics (--check) ──────────────────────────────────────────────────

#[derive(Debug)]
enum PluginStatus {
    Ready { lines: usize },
    Error(String),
}

fn diagnose_plugins(plugins_dir: &std::path::Path, data_dir: &PathBuf) -> Vec<(String, PluginStatus)> {
    let plugins = manifest::load_plugins_from_dir(plugins_dir);
    let mut results = Vec::new();
    let agent_version = env!("CARGO_PKG_VERSION");
    for plugin in &plugins {
        let id = plugin.manifest.id.clone();
        if EXCLUDED_PLUGINS.contains(&id.as_str()) {
            continue;
        }
        let output = runtime::run_probe(plugin, data_dir, agent_version);
        let is_error = output.lines.len() == 1
            && matches!(
                &output.lines[0],
                runtime::MetricLine::Badge { label, .. } if label == "Error"
            );
        if is_error {
            let msg = match &output.lines[0] {
                runtime::MetricLine::Badge { text, .. } => text.clone(),
                _ => "unknown".into(),
            };
            results.push((id, PluginStatus::Error(msg)));
        } else {
            results.push((id, PluginStatus::Ready { lines: output.lines.len() }));
        }
    }
    results
}

async fn check_relay_health(client: &reqwest::Client, relay_url: &str) -> Result<(), String> {
    let url = format!("{}/v1/health", relay_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("network unreachable: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

async fn check_relay_auth(
    client: &reqwest::Client,
    relay_url: &str,
    token: &str,
) -> Result<usize, String> {
    let url = format!("{}/v1/pull", relay_url.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("network unreachable: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} (token rejected?)", resp.status()));
    }
    let body = resp.text().await.map_err(|e| format!("read response: {}", e))?;
    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("invalid relay response: {}", e))?;
    let count = v.get("machines").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
    Ok(count)
}

async fn run_doctor(
    cli: &Cli,
    token: &str,
    relay: &str,
    client: &reqwest::Client,
    plugins_dir: &PathBuf,
    data_dir: &PathBuf,
) -> i32 {
    let mut all_ok = true;

    println!("\n=== OpenUsage Agent Diagnostic ===");
    println!("Machine: {}", cli.machine_name.clone().unwrap_or_else(resolve_machine_id));
    println!("Source:  {}", cli.source);
    println!("Relay:   {}", relay);
    println!("Token:   {}...{}", &token[..token.len().min(8)], &token[token.len().saturating_sub(4)..]);
    println!();

    println!("[1/4] Relay reachable?");
    match check_relay_health(client, relay).await {
        Ok(()) => println!("       OK"),
        Err(e) => {
            println!("       FAIL: {}", e);
            println!("       Hint: check the relay URL is correct and the host is up");
            all_ok = false;
        }
    }

    println!("[2/4] Token accepted by relay?");
    match check_relay_auth(client, relay, token).await {
        Ok(n) => println!("       OK (relay reports {} machine(s) for this token)", n),
        Err(e) => {
            println!("       FAIL: {}", e);
            println!("       Hint: re-copy the token from the dashboard's Settings -> Multi-Machine Sync");
            all_ok = false;
        }
    }

    println!("[3/4] Plugins available?");
    if cli.source == "probe" {
        if !plugins_dir.exists() {
            println!("       Extracting bundled plugins...");
            if let Err(e) = extract_bundled_plugins(plugins_dir) {
                println!("       FAIL: {}", e);
                all_ok = false;
            }
        }
        let plugin_count = std::fs::read_dir(plugins_dir)
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        println!("       OK ({} plugins in {})", plugin_count, plugins_dir.display());
    } else {
        println!("       Skipped (source mode is '{}')", cli.source);
    }

    println!("[4/4] Plugin readiness:");
    if cli.source == "probe" {
        let results = diagnose_plugins(plugins_dir, data_dir);
        let mut ready_count = 0;
        let mut error_count = 0;
        for (id, status) in &results {
            match status {
                PluginStatus::Ready { lines } => {
                    println!("       OK    {} ({} lines)", id, lines);
                    ready_count += 1;
                }
                PluginStatus::Error(msg) => {
                    let short = if msg.len() > 80 { &msg[..80] } else { msg };
                    println!("       SKIP  {} ({})", id, short);
                    error_count += 1;
                }
            }
        }
        println!();
        println!("Summary: {} ready, {} skipped (no creds / not installed)", ready_count, error_count);
        if ready_count == 0 {
            println!("Hint: at least one supported AI tool (Claude Code, Cursor, Codex, etc.) needs to be");
            println!("      installed and logged in on this machine for it to contribute usage data.");
        }
    } else if cli.source == "local-api" {
        let api = cli
            .local_api
            .clone()
            .unwrap_or_else(|| DEFAULT_LOCAL_API.to_string());
        match collect_via_local_api(client, &api).await {
            Ok(snaps) => println!("       Local API returned {} snapshots", snaps.len()),
            Err(e) => {
                println!("       FAIL: {}", e);
                all_ok = false;
            }
        }
    } else {
        println!("       (source={}; nothing to probe)", cli.source);
    }

    println!();
    if all_ok {
        println!("All checks passed. Re-start the service to begin pushing.");
        0
    } else {
        println!("Some checks failed. See hints above.");
        1
    }
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
    let cli = Cli::parse();

    // In --check mode, silence plugin engine logs so the diagnostic output
    // isn't drowned in DEBUG-level keychain misses, sqlite errors, etc.
    let default_filter = if cli.check {
        "error,openusage_agent=info"
    } else {
        "info"
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter)).init();

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

    // Resolve token & relay: explicit flags take priority; otherwise fall back
    // to the saved config from a previous run (so `openusage-agent --check`
    // works without retyping anything).
    let saved = load_config(&data_dir);
    let token = cli
        .token
        .clone()
        .or_else(|| saved.as_ref().map(|c| c.token.clone()));
    let relay = cli
        .relay
        .clone()
        .or_else(|| saved.as_ref().map(|c| c.relay.clone()));

    let token = match token {
        Some(t) => t,
        None => {
            eprintln!("error: --token required (or run from a directory with a saved config)");
            std::process::exit(2);
        }
    };
    let relay = match relay {
        Some(r) => r,
        None => {
            eprintln!("error: --relay required (or run from a directory with a saved config)");
            std::process::exit(2);
        }
    };

    // Persist for next run (so --check works without flags)
    save_config(&data_dir, &token, &relay);

    log::info!(
        "openusage-agent starting: machine={}, relay={}, source={}, interval={}s",
        machine_name, relay, cli.source, cli.interval
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

    // One-shot diagnostic mode
    if cli.check {
        let code = run_doctor(&cli, &token, &relay, &client, &plugins_dir, &data_dir).await;
        std::process::exit(code);
    }

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
        // Always push, even with 0 snapshots, so the dashboard sees the
        // machine as "connected" (instead of completely invisible).
        let push = MachinePush {
            machine_id: machine_id.clone(),
            machine_name: machine_name.clone(),
            snapshots,
            pushed_at: now_rfc3339(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        match push_to_relay(&client, &relay, &token, &push).await {
            Ok(()) => log::info!("pushed to relay"),
            Err(e) => log::error!("push failed: {}", e),
        }
    }
}
