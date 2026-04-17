use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
};
use openusage_shared::{MachineEntry, MachinePush, SyncPullResponse};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ─── State ──────────────────────────────────────────────────────────────────

type RoomMap = HashMap<String, HashMap<String, MachineEntry>>;

#[derive(Clone)]
struct AppState {
    rooms: Arc<RwLock<RoomMap>>,
}

// ─── Auth helper ────────────────────────────────────────────────────────────

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn now_rfc3339() -> String {
    let now = time::OffsetDateTime::now_utc();
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({"ok": true}))
}

async fn push_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let Some(token) = extract_bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing or invalid Authorization header").into_response();
    };

    let push: MachinePush = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("invalid JSON: {}", e)).into_response();
        }
    };

    if push.machine_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "machine_id is required").into_response();
    }

    let entry = MachineEntry {
        machine_id: push.machine_id.clone(),
        machine_name: push.machine_name,
        snapshots: push.snapshots,
        pushed_at: push.pushed_at,
        last_seen_at: now_rfc3339(),
    };

    log::info!(
        "push: token={}..., machine={}, snapshots={}",
        &token[..token.len().min(8)],
        entry.machine_id,
        entry.snapshots.len()
    );

    let mut rooms = state.rooms.write().await;
    let room = rooms.entry(token).or_default();

    // Rate limit: max 120 machines per token (prevent abuse)
    if !room.contains_key(&entry.machine_id) && room.len() >= 120 {
        return (StatusCode::TOO_MANY_REQUESTS, "too many machines for this token").into_response();
    }

    room.insert(entry.machine_id.clone(), entry);

    StatusCode::OK.into_response()
}

async fn pull_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(token) = extract_bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing or invalid Authorization header").into_response();
    };

    let rooms = state.rooms.read().await;
    let machines = match rooms.get(&token) {
        Some(room) => room.values().cloned().collect(),
        None => vec![],
    };

    axum::Json(SyncPullResponse { machines }).into_response()
}

async fn delete_machine_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(machine_id): Path<String>,
) -> impl IntoResponse {
    let Some(token) = extract_bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED;
    };

    let mut rooms = state.rooms.write().await;
    if let Some(room) = rooms.get_mut(&token) {
        room.remove(&machine_id);
        if room.is_empty() {
            rooms.remove(&token);
        }
    }

    StatusCode::NO_CONTENT
}

// ─── Stale cleanup ──────────────────────────────────────────────────────────

async fn cleanup_stale_entries(state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(600)); // 10 min
    loop {
        interval.tick().await;

        let mut rooms = state.rooms.write().await;
        let mut empty_tokens = vec![];

        for (token, room) in rooms.iter_mut() {
            // Remove machines not seen in 24 hours
            let cutoff = time::OffsetDateTime::now_utc() - time::Duration::hours(24);
            let cutoff_str = cutoff
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default();

            room.retain(|_, entry| entry.last_seen_at > cutoff_str);

            if room.is_empty() {
                empty_tokens.push(token.clone());
            }
        }

        for token in empty_tokens {
            rooms.remove(&token);
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let port: u16 = std::env::var("RELAY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8090);

    let state = AppState {
        rooms: Arc::new(RwLock::new(HashMap::new())),
    };

    // Spawn stale entry cleanup task
    tokio::spawn(cleanup_stale_entries(state.clone()));

    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/push", post(push_handler))
        .route("/v1/pull", get(pull_handler))
        .route("/v1/machines/{machine_id}", delete(delete_machine_handler))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(256 * 1024)) // 256 KB
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    log::info!("openusage-relay listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
