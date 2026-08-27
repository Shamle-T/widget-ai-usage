use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{
    registry::find_executable_with_extensions, ProviderAdapter, ProviderAvailability,
    ProviderError, ProviderUsage, UsageWindow, UsageWindowKind,
};

const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_RESPONSE_LINES: usize = 16;
const MAX_RESPONSE_LINE_BYTES: usize = 64 * 1024;

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
        if resolve_codex_executable().is_some() {
            ProviderAvailability::Detected
        } else {
            ProviderAvailability::NotInstalled
        }
    }

    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError> {
        let executable = resolve_codex_executable().ok_or(ProviderError::NotInstalled)?;

        // This is the documented, read-only Codex app-server flow. It creates no thread or turn.
        let mut child = Command::new(executable)
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

#[cfg(not(target_os = "windows"))]
fn resolve_codex_executable() -> Option<PathBuf> {
    find_executable_with_extensions("codex", &[""])
}

#[cfg(target_os = "windows")]
fn resolve_codex_executable() -> Option<PathBuf> {
    find_executable_with_extensions("codex", &[".exe"]).or_else(|| {
        let shim = find_executable_with_extensions("codex", &[".cmd"])?;
        npm_codex_candidates(&shim)
            .into_iter()
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(target_os = "windows")]
fn npm_codex_candidates(shim: &Path) -> Vec<PathBuf> {
    let Some(bin_directory) = shim.parent() else {
        return Vec::new();
    };
    let (package, target) = if cfg!(target_arch = "aarch64") {
        ("codex-win32-arm64", "aarch64-pc-windows-msvc")
    } else {
        ("codex-win32-x64", "x86_64-pc-windows-msvc")
    };
    let package_root = bin_directory.join("node_modules").join("@openai");
    let vendor_tail = Path::new("vendor")
        .join(target)
        .join("bin")
        .join("codex.exe");
    vec![
        package_root
            .join("codex")
            .join("node_modules")
            .join("@openai")
            .join(package)
            .join(&vendor_tail),
        package_root.join(package).join(&vendor_tail),
        package_root.join("codex").join(&vendor_tail),
    ]
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

    #[cfg(target_os = "windows")]
    use super::npm_codex_candidates;
    #[cfg(target_os = "windows")]
    use std::path::Path;

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

    #[cfg(target_os = "windows")]
    #[test]
    fn resolves_the_official_npm_native_binary_layout_without_a_shell() {
        let target = if cfg!(target_arch = "aarch64") {
            "aarch64-pc-windows-msvc"
        } else {
            "x86_64-pc-windows-msvc"
        };
        let candidates =
            npm_codex_candidates(Path::new(r"C:\Users\test\AppData\Roaming\npm\codex.cmd"));
        assert!(candidates.iter().any(|candidate| {
            candidate.ends_with(
                Path::new("vendor")
                    .join(target)
                    .join("bin")
                    .join("codex.exe"),
            )
        }));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.extension().is_some_and(|ext| ext == "exe")));
    }
}
