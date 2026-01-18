use crate::config::Config;
use std::collections::HashMap;

pub async fn report_error(error_type: &str, error_message: &str) {
    let config = Config::get_webhook_config();
    
    if !config.enabled || config.url.is_empty() {
        return;
    }

    let hostname = gethostname::gethostname()
        .to_string_lossy()
        .to_string();
    
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "Unknown".to_string());

    let content = format!(
        "## Kurinium Error Report\n\n\
        **Error Type:** `{}`\n\
        **Device:** `{}@{}`\n\
        **Message:**\n```\n{}\n```\n\n\
        *Please report back to Kukuri*",
        error_type,
        username,
        hostname,
        error_message
    );

    let mut payload = HashMap::new();
    payload.insert("content", content);

    let _ = reqwest::Client::new()
        .post(&config.url)
        .json(&payload)
        .send()
        .await;
}

pub async fn report_gateway_error(code: u16, reason: &str) {
    report_error(
        &format!("Gateway Close ({})", code),
        reason
    ).await;
}

pub async fn report_crash(error: &str) {
    report_error("Fatal Crash", error).await;
}

pub async fn report_anti_analysis(detections: &[String]) {
    let details = detections.join("\n");
    report_error("Anti-Analysis Triggered", &details).await;
}
