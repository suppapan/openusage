use openusage_shared::SyncPullResponse;

/// Pull remote machine data from the relay server.
pub fn pull_remote_machines(
    relay_url: &str,
    token: &str,
) -> Result<SyncPullResponse, String> {
    let url = format!("{}/v1/pull", relay_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {}", e))?;

    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .map_err(|e| format!("failed to reach relay: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("relay returned {}: {}", status, body));
    }

    let body = resp
        .text()
        .map_err(|e| format!("failed to read relay response: {}", e))?;

    serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse relay response: {}", e))
}

/// Delete a machine entry from the relay.
pub fn delete_remote_machine(
    relay_url: &str,
    token: &str,
    machine_id: &str,
) -> Result<(), String> {
    let url = format!(
        "{}/v1/machines/{}",
        relay_url.trim_end_matches('/'),
        machine_id
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {}", e))?;

    let resp = client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .map_err(|e| format!("failed to reach relay: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("relay returned {}: {}", status, body));
    }

    Ok(())
}
