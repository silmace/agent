use tracing::debug;

pub async fn add_agent(
    api_server: String,
    port: u16,
    api_key: String,
    endpoint: Option<String>,
    version: &str,
    ip_type: &str,
    init: bool,
) -> bool {
    let res = reqwest::Client::new()
        .post(api_server)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "port": port,
            "version": version,
            "endpoint": endpoint,
            "init": init,
        }))
        .send()
        .await;
    return match res {
        Ok(res) => {
            let success = res.status().is_success();
            if success {
                debug!("add {} agent success", ip_type);
                true
            } else {
                debug!(
                    "add {} agent failed: {}",
                    ip_type,
                    res.text().await.unwrap()
                );
                false
            }
        }
        Err(e) => {
            debug!("add {} agent failed: {}", ip_type, e);
            false
        }
    };
}
