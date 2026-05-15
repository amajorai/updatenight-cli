use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::api::api_base;
use crate::config;

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

pub async fn device_login() -> Result<()> {
    let client = reqwest::Client::new();
    let base = api_base();

    let resp: DeviceCodeResponse = client
        .post(format!("{}/api/auth/device/code", base))
        .json(&serde_json::json!({ "client_id": "updatenight-cli" }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("\n  Your code:    {}", resp.user_code);
    println!("  Authorize at: {}", resp.verification_uri);
    println!("\nOpening browser...\n");

    let verify_url = format!("{}?user_code={}", resp.verification_uri, resp.user_code);
    let _ = open::that(&verify_url);

    let poll_interval = Duration::from_secs(resp.interval.unwrap_or(5).max(5));
    let deadline = Instant::now() + Duration::from_secs(resp.expires_in);

    loop {
        if Instant::now() > deadline {
            return Err(anyhow!("Authorization timed out. Try `un login` again."));
        }

        tokio::time::sleep(poll_interval).await;

        let result: TokenResponse = client
            .post(format!("{}/api/auth/device/token", base))
            .json(&serde_json::json!({
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
                "device_code": resp.device_code,
                "client_id": "updatenight-cli"
            }))
            .send()
            .await?
            .json()
            .await?;

        match (result.access_token, result.error.as_deref()) {
            (Some(token), _) => {
                let mut cfg = config::load();
                cfg.token = Some(token);
                config::save(&cfg)?;
                println!("\nAuthorized! Run `un` to open the catalog.");
                return Ok(());
            }
            (_, Some("authorization_pending")) => {
                print!(".");
                std::io::stdout().flush().ok();
            }
            (_, Some("slow_down")) => {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            (_, Some("expired_token")) => return Err(anyhow!("Code expired.")),
            (_, Some("access_denied")) => return Err(anyhow!("Access denied.")),
            _ => {}
        }
    }
}
