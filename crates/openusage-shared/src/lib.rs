use serde::{Deserialize, Serialize};

// ─── Metric types (mirror of plugin_engine::runtime, serde-only) ────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProgressFormat {
    Percent,
    Dollars,
    Count { suffix: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MetricLine {
    Text {
        label: String,
        value: String,
        color: Option<String>,
        subtitle: Option<String>,
    },
    Progress {
        label: String,
        used: f64,
        limit: f64,
        format: ProgressFormat,
        #[serde(rename = "resetsAt")]
        resets_at: Option<String>,
        #[serde(rename = "periodDurationMs")]
        period_duration_ms: Option<u64>,
        color: Option<String>,
    },
    Badge {
        label: String,
        text: String,
        color: Option<String>,
        subtitle: Option<String>,
    },
}

// ─── Plugin snapshot (wire-compatible with CachedPluginSnapshot) ────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSnapshot {
    pub provider_id: String,
    pub display_name: String,
    pub plan: Option<String>,
    pub lines: Vec<MetricLine>,
    pub fetched_at: String,
}

// ─── Multi-machine sync wire types ──────────────────────────────────────────

/// Payload an agent pushes to the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachinePush {
    pub machine_id: String,
    pub machine_name: String,
    pub snapshots: Vec<PluginSnapshot>,
    pub pushed_at: String,
    pub agent_version: String,
}

/// What the relay stores and returns per machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineEntry {
    pub machine_id: String,
    pub machine_name: String,
    pub snapshots: Vec<PluginSnapshot>,
    pub pushed_at: String,
    pub last_seen_at: String,
}

/// Response when the dashboard pulls from the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPullResponse {
    pub machines: Vec<MachineEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_push_roundtrips_json() {
        let push = MachinePush {
            machine_id: "ubuntu-server-01".into(),
            machine_name: "Ubuntu Server".into(),
            snapshots: vec![PluginSnapshot {
                provider_id: "claude".into(),
                display_name: "Claude".into(),
                plan: Some("Pro".into()),
                lines: vec![MetricLine::Progress {
                    label: "Session".into(),
                    used: 42.0,
                    limit: 100.0,
                    format: ProgressFormat::Percent,
                    resets_at: Some("2026-04-18T00:00:00Z".into()),
                    period_duration_ms: None,
                    color: None,
                }],
                fetched_at: "2026-04-17T12:00:00Z".into(),
            }],
            pushed_at: "2026-04-17T12:00:01Z".into(),
            agent_version: "0.1.0".into(),
        };

        let json = serde_json::to_string(&push).unwrap();
        let parsed: MachinePush = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.machine_id, "ubuntu-server-01");
        assert_eq!(parsed.snapshots.len(), 1);
        assert_eq!(parsed.snapshots[0].provider_id, "claude");
    }

    #[test]
    fn sync_pull_response_roundtrips_json() {
        let resp = SyncPullResponse {
            machines: vec![MachineEntry {
                machine_id: "m1".into(),
                machine_name: "MacBook".into(),
                snapshots: vec![],
                pushed_at: "2026-04-17T12:00:00Z".into(),
                last_seen_at: "2026-04-17T12:00:05Z".into(),
            }],
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SyncPullResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.machines.len(), 1);
        assert_eq!(parsed.machines[0].machine_name, "MacBook");
    }
}
