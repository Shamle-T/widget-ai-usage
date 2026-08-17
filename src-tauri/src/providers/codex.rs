use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{
    registry::executable_exists, ProviderAdapter, ProviderAvailability, ProviderError,
    ProviderUsage, UsageWindow, UsageWindowKind,
};

const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_RESPONSE_LINES: usize = 16;
const MAX_RESPONSE_LINE_BYTES: usize = 64 * 1024;

#[cfg(target_os = "windows")]
const CODEX_COMMAND: &str = "codex.cmd";
#[cfg(not(target_os = "windows"))]
const CODEX_COMMAND: &str = "codex";

pub struct CodexProvider;

impl ProviderAdapter for CodexProvider {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn source_name(&self) -> &'static str {
        "Codex app-server"
    }

    fn detect(&self) -> ProviderAvailability {
        if executable_exists(CODEX_COMMAND) {
            ProviderAvailability::Detected
        } else {
            ProviderAvailability::NotInstalled
        }
    }

    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError> {
        if matches!(self.detect(), ProviderAvailability::NotInstalled) {
            return Err(ProviderError::NotInstalled);
        }

        // This is the documented, read-only Codex app-server flow. It creates no thread or turn.
        let mut child = Command::new(CODEX_COMMAND)
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ProviderError::RefreshFailed)?;

        let mut stdin = child.stdin.take().ok_or(ProviderError::RefreshFailed)?;
        let stdout = child.stdout.take().ok_or(ProviderError::RefreshFailed)?;
        let (sender, receiver) = mpsc::sync_channel(2);
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            for _ in 0..MAX_RESPONSE_LINES {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() > MAX_RESPONSE_LINE_BYTES => {
                        let _ = sender.send(Err(ProviderError::MalformedResponse));
                        return;
                    }
                    Ok(_) => match serde_json::from_str::<RpcResponse>(&line) {
                        Ok(response) if matches!(response.id, Some(1) | Some(2)) => {
                            let is_rate_limit_response = response.id == Some(2);
                            if sender.send(Ok(response)).is_err() {
                                return;
                            }
                            if is_rate_limit_response {
                                return;
                            }
                        }
                        Ok(_) => continue,
                        Err(_) => continue,
                    },
                    Err(_) => break,
                }
            }
            let _ = sender.send(Err(ProviderError::MalformedResponse));
        });

        let result = (|| -> Result<ProviderUsage, ProviderError> {
            stdin
                .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"limitlens\",\"title\":\"LimitLens\",\"version\":\"0.1.0\"}}}\n")
                .and_then(|_| stdin.flush())
                .map_err(|_| ProviderError::RefreshFailed)?;

            let initialized = receiver
                .recv_timeout(APP_SERVER_TIMEOUT)
                .unwrap_or(Err(ProviderError::RefreshFailed))?;
            if initialized.id != Some(1) || initialized.error.is_some() {
                return Err(ProviderError::RefreshFailed);
            }

            stdin
                .write_all(
                    b"{\"jsonrpc\":\"2.0\",\"method\":\"initialized\",\"params\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"account/rateLimits/read\"}\n",
                )
                .and_then(|_| stdin.flush())
                .map_err(|_| ProviderError::RefreshFailed)?;

            receiver
                .recv_timeout(APP_SERVER_TIMEOUT)
                .unwrap_or(Err(ProviderError::RefreshFailed))?
                .into_rate_limits()
                .and_then(parse_rate_limits)
        })();
        drop(stdin);
        let _ = child.kill();
        let _ = child.wait();
        let _ = reader.join();

        result
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    id: Option<u64>,
    result: Option<RateLimitsResponse>,
    error: Option<RpcError>,
}

impl RpcResponse {
    fn into_rate_limits(self) -> Result<RateLimitsResponse, ProviderError> {
        if let Some(error) = self.error {
            let message = error.message.to_ascii_lowercase();
            return Err(if message.contains("auth") || message.contains("logged") {
                ProviderError::AuthenticationRequired
            } else {
                ProviderError::RefreshFailed
            });
        }
        self.result.ok_or(ProviderError::MalformedResponse)
    }
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsResponse {
    rate_limits: Option<RateLimitSnapshot>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitSnapshot {
    primary: Option<RateLimitWindow>,
    secondary: Option<RateLimitWindow>,
    plan_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitWindow {
    used_percent: Option<f64>,
    window_duration_mins: Option<u64>,
    resets_at: Option<i64>,
}

fn parse_rate_limits(response: RateLimitsResponse) -> Result<ProviderUsage, ProviderError> {
    let snapshot = response.rate_limits.ok_or(ProviderError::Unsupported)?;
    let mut windows = Vec::new();
    if let Some(primary) = snapshot.primary {
        windows.push(normalize_window(
            "primary",
            primary,
            UsageWindowKind::ShortTerm,
            false,
        ));
    }
    if let Some(secondary) = snapshot.secondary {
        windows.push(normalize_window(
            "secondary",
            secondary,
            UsageWindowKind::Weekly,
            true,
        ));
    }
    if windows.is_empty() {
        return Err(ProviderError::Unsupported);
    }

    Ok(ProviderUsage {
        provider_id: "codex".to_owned(),
        provider_name: "Codex".to_owned(),
        account_label: snapshot.plan_type,
        windows,
        updated_at: Utc::now(),
        source: "Codex app-server".to_owned(),
    })
}

fn normalize_window(
    id: &str,
    window: RateLimitWindow,
    kind: UsageWindowKind,
    force_weekly_label: bool,
) -> UsageWindow {
    let display_name = if force_weekly_label {
        "Weekly".to_owned()
    } else {
        duration_label(window.window_duration_mins)
    };
    UsageWindow {
        id: id.to_owned(),
        display_name,
        kind,
        used_percent: window.used_percent,
        resets_at: window
            .resets_at
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0)),
    }
}

fn duration_label(minutes: Option<u64>) -> String {
    match minutes {
        Some(minutes) if minutes >= 60 && minutes % 60 == 0 => {
            format!("{} hour", minutes / 60)
        }
        Some(minutes) => format!("{minutes} min"),
        None => "Short term".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_rate_limits, RateLimitsResponse};

    #[test]
    fn parses_codex_consumed_percentages_without_inversion() {
        let response: RateLimitsResponse = serde_json::from_str(
            r#"{"rateLimits":{"primary":{"usedPercent":24,"windowDurationMins":300,"resetsAt":1760000000},"secondary":{"usedPercent":61,"windowDurationMins":10080,"resetsAt":1760500000},"planType":"plus"}}"#,
        )
        .expect("fixture is valid");

        let usage = parse_rate_limits(response).expect("rate limits parse");
        assert_eq!(usage.account_label.as_deref(), Some("plus"));
        assert_eq!(usage.windows[0].used_percent, Some(24.0));
        assert_eq!(usage.windows[0].display_name, "5 hour");
        assert_eq!(usage.windows[1].used_percent, Some(61.0));
        assert_eq!(usage.windows[1].display_name, "Weekly");
    }

    #[test]
    fn rejects_empty_rate_limit_response() {
        let response: RateLimitsResponse =
            serde_json::from_str(r#"{"rateLimits":null}"#).expect("fixture is valid");
        assert!(parse_rate_limits(response).is_err());
    }
}
