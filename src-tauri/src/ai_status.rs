//! Status of running AI coding assistants (Claude Code in a terminal, VS Code or Claude Desktop,
//! and GitHub Copilot in VS Code), plus their plan usage and reset times.
//!
//! Sessions are read from the files Claude Code keeps in `~/.claude/sessions`. Claude usage comes
//! from Anthropic's OAuth usage endpoint using the token Claude Code already stores locally; it is
//! sent only to Anthropic. Copilot usage needs a GitHub token (saved in Settings, or from `gh`).

use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessesToUpdate, System};

const USAGE_TTL: Duration = Duration::from_secs(60);
const HTTP_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSession {
    pub pid: u32,
    pub session_id: String,
    pub name: String,
    pub cwd: String,
    /// `claude-vscode`, `cli`, `claude-desktop`, ...
    pub entrypoint: String,
    /// `busy`, `idle`, `waiting`, ...
    pub status: String,
    pub version: String,
    pub started_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSessions {
    pub claude_sessions: Vec<ClaudeSession>,
    pub claude_desktop_running: bool,
    pub vscode_running: bool,
    pub copilot_installed: bool,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub label: String,
    /// Percent of the limit used, 0-100.
    pub percent_used: f64,
    /// Plain detail such as "120 of 300 left", when the API gives absolute numbers.
    pub detail: Option<String>,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    pub plan: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsage {
    pub claude: UsageReport,
    pub copilot: UsageReport,
    pub copilot_token_saved: bool,
}

// ---------------------------------------------------------------------------------------------
// Sessions and process detection
// ---------------------------------------------------------------------------------------------

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn process_name_matches(name: &str, wanted: &[&str]) -> bool {
    let name = name.to_ascii_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    wanted.iter().any(|candidate| name == *candidate)
}

/// VS Code's per-user data folders (`Code`, `Code - Insiders`) for this platform.
fn vscode_user_dirs() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        Some(home.join("Library").join("Application Support"))
    } else {
        Some(home.join(".config"))
    };
    base.map(|base| {
        ["Code", "Code - Insiders"]
            .iter()
            .map(|name| base.join(name).join("User"))
            .collect()
    })
    .unwrap_or_default()
}

/// Recent VS Code builds ship Copilot Chat built in, so there is no folder in `~/.vscode/extensions`.
/// Its data folder under `globalStorage` exists either way, so check both places.
fn copilot_extension_installed() -> bool {
    let in_global_storage = vscode_user_dirs()
        .iter()
        .any(|user| user.join("globalStorage").join("github.copilot-chat").is_dir());
    if in_global_storage {
        return true;
    }
    let Some(home) = home_dir() else {
        return false;
    };
    [".vscode", ".vscode-insiders"].iter().any(|dir| {
        std::fs::read_dir(home.join(dir).join("extensions"))
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with("github.copilot")
                })
            })
            .unwrap_or(false)
    })
}

fn read_sessions(system: &System) -> Vec<ClaudeSession> {
    let Some(dir) = home_dir().map(|home| home.join(".claude").join("sessions")) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut sessions: Vec<ClaudeSession> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|raw| serde_json::from_str::<Value>(&raw).ok())
        .filter_map(|value| {
            let pid = number(&value, "pid") as u32;
            // Session files outlive crashed processes, so only list ones that are still running.
            if pid == 0 || system.process(sysinfo::Pid::from_u32(pid)).is_none() {
                return None;
            }
            let cwd = text(&value, "cwd");
            let name = match text(&value, "name") {
                name if !name.is_empty() => name,
                _ => cwd
                    .rsplit(['\\', '/'])
                    .find(|part| !part.is_empty())
                    .unwrap_or("Claude Code")
                    .to_string(),
            };
            Some(ClaudeSession {
                pid,
                session_id: text(&value, "sessionId"),
                name,
                cwd,
                entrypoint: text(&value, "entrypoint"),
                status: text(&value, "status"),
                version: text(&value, "version"),
                started_at: number(&value, "startedAt"),
                updated_at: number(&value, "updatedAt"),
            })
        })
        .collect();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions
}

#[tauri::command]
pub fn get_ai_sessions() -> AiSessions {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut claude_desktop_running = false;
    let mut vscode_running = false;
    for process in system.processes().values() {
        let name = process.name().to_string_lossy();
        // The Claude Code CLI binary is also called claude.exe, so tell them apart by install path.
        claude_desktop_running |= process_name_matches(&name, &["claude"])
            && process.exe().is_some_and(|path| {
                let path = path.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
                path.contains("\\windowsapps\\claude_")
                    || path.contains("\\anthropicclaude\\")
                    || path.contains("\\applications\\claude.app\\")
            });
        vscode_running |= process_name_matches(&name, &["code", "code - insiders"]);
    }

    AiSessions {
        claude_sessions: read_sessions(&system),
        claude_desktop_running,
        vscode_running,
        copilot_installed: copilot_extension_installed(),
        now_ms: now_ms(),
    }
}

// ---------------------------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------------------------

struct Cached {
    at: Instant,
    usage: AiUsage,
}

fn cache() -> &'static Mutex<Option<Cached>> {
    static CACHE: std::sync::OnceLock<Mutex<Option<Cached>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent("AstroDeck")
        .build()
        .map_err(|error| error.to_string())
}

fn failed(error: impl Into<String>) -> UsageReport {
    UsageReport {
        plan: None,
        windows: Vec::new(),
        error: Some(error.into()),
    }
}

fn claude_window_label(key: &str) -> Option<&'static str> {
    Some(match key {
        "five_hour" => "5-hour session",
        "seven_day" => "Weekly",
        "seven_day_opus" => "Weekly · Opus",
        "seven_day_sonnet" => "Weekly · Sonnet",
        _ => return None,
    })
}

fn fetch_claude_usage() -> UsageReport {
    let Some(path) = home_dir().map(|home| home.join(".claude").join(".credentials.json")) else {
        return failed("Home folder not found");
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return failed("Claude Code is not signed in on this computer");
    };
    let Ok(credentials) = serde_json::from_str::<Value>(&raw) else {
        return failed("Claude Code credentials could not be read");
    };
    let oauth = &credentials["claudeAiOauth"];
    let Some(token) = oauth.get("accessToken").and_then(Value::as_str) else {
        return failed("Claude Code is not signed in with a Claude account");
    };
    let plan = oauth
        .get("subscriptionType")
        .and_then(Value::as_str)
        .map(str::to_string);
    if oauth
        .get("expiresAt")
        .and_then(Value::as_u64)
        .is_some_and(|expires| expires <= now_ms())
    {
        return UsageReport {
            plan,
            ..failed("Claude login expired. Open Claude Code once to refresh it")
        };
    }

    let client = match http() {
        Ok(client) => client,
        Err(error) => return failed(error),
    };
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .send();
    let body: Value = match response {
        Ok(response) if response.status().is_success() => match response.json() {
            Ok(body) => body,
            Err(_) => return UsageReport { plan, ..failed("Unexpected usage response") },
        },
        Ok(response) => {
            return UsageReport {
                plan,
                ..failed(format!("Usage request failed ({})", response.status()))
            }
        }
        Err(_) => return UsageReport { plan, ..failed("Could not reach Anthropic") },
    };

    let mut windows = Vec::new();
    if let Some(object) = body.as_object() {
        for (key, value) in object {
            let (Some(label), Some(percent)) = (
                claude_window_label(key),
                value.get("utilization").and_then(Value::as_f64),
            ) else {
                continue;
            };
            windows.push(UsageWindow {
                label: label.to_string(),
                percent_used: percent.clamp(0.0, 100.0),
                detail: None,
                resets_at: value
                    .get("resets_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
    }
    windows.sort_by_key(|window| match window.label.as_str() {
        "5-hour session" => 0,
        "Weekly" => 1,
        _ => 2,
    });
    UsageReport {
        plan,
        windows,
        error: None,
    }
}

fn github_cli_token() -> Option<String> {
    let mut command = Command::new("gh");
    command.args(["auth", "token"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command.output().ok()?;
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !token.is_empty()).then_some(token)
}

fn copilot_window_label(key: &str) -> Option<&'static str> {
    Some(match key {
        "premium_interactions" => "Premium requests",
        "chat" => "Chat",
        "completions" => "Completions",
        _ => return None,
    })
}

/// 19536 -> "19,536".
fn grouped(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    if value < 0 {
        out.insert(0, '-');
    }
    out
}

fn fetch_copilot_usage(saved_token: &str) -> UsageReport {
    let token = if saved_token.is_empty() {
        match github_cli_token() {
            Some(token) => token,
            None => return failed("Add a GitHub token in Settings → AI status"),
        }
    } else {
        saved_token.to_string()
    };

    let client = match http() {
        Ok(client) => client,
        Err(error) => return failed(error),
    };
    let response = client
        .get("https://api.github.com/copilot_internal/user")
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/json")
        .header("editor-version", "vscode/1.99.0")
        .header("editor-plugin-version", "copilot-chat/0.26.0")
        .send();
    let body: Value = match response {
        Ok(response) if response.status().is_success() => match response.json() {
            Ok(body) => body,
            Err(_) => return failed("Unexpected Copilot response"),
        },
        Ok(response) if response.status().as_u16() == 401 || response.status().as_u16() == 403 => {
            return failed("GitHub rejected the token, or it has no Copilot access")
        }
        Ok(response) => return failed(format!("Copilot request failed ({})", response.status())),
        Err(_) => return failed("Could not reach GitHub"),
    };

    let plan = body
        .get("copilot_plan")
        .and_then(Value::as_str)
        .map(str::to_string);
    let resets_at = body
        .get("quota_reset_date")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut windows = Vec::new();
    if let Some(snapshots) = body.get("quota_snapshots").and_then(Value::as_object) {
        for (key, value) in snapshots {
            let Some(mut label) = copilot_window_label(key) else {
                continue;
            };
            // Token-billed plans count credits instead of premium requests.
            let credits = value.get("token_based_billing").and_then(Value::as_bool) == Some(true)
                && key == "premium_interactions";
            if credits {
                label = "AI credits";
            }
            if value.get("unlimited").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let Some(remaining_percent) = value.get("percent_remaining").and_then(Value::as_f64)
            else {
                continue;
            };
            let detail = match (
                value.get("remaining").and_then(Value::as_f64),
                value.get("entitlement").and_then(Value::as_f64),
            ) {
                (Some(remaining), Some(entitlement)) => Some(format!(
                    "{} of {}{} left",
                    grouped(remaining.round() as i64),
                    grouped(entitlement.round() as i64),
                    if credits { " credits" } else { "" }
                )),
                _ => None,
            };
            windows.push(UsageWindow {
                label: label.to_string(),
                percent_used: (100.0 - remaining_percent).clamp(0.0, 100.0),
                detail,
                resets_at: resets_at.clone(),
            });
        }
    }
    windows.sort_by_key(|window| u8::from(!matches!(window.label.as_str(), "Premium requests" | "AI credits")));
    UsageReport {
        plan,
        windows,
        error: None,
    }
}

/// Fetches fresh numbers at most once a minute; callers in between get the previous result.
#[tauri::command]
pub async fn get_ai_usage(app: tauri::AppHandle, force: Option<bool>) -> Result<AiUsage, String> {
    if force != Some(true) {
        if let Some(cached) = cache().lock().map_err(|error| error.to_string())?.as_ref() {
            if cached.at.elapsed() < USAGE_TTL {
                return Ok(cached.usage.clone());
            }
        }
    }

    let saved_token = crate::prefs::copilot_token(&app);
    let usage = tauri::async_runtime::spawn_blocking(move || AiUsage {
        claude: fetch_claude_usage(),
        copilot: fetch_copilot_usage(&saved_token),
        copilot_token_saved: !saved_token.is_empty(),
    })
    .await
    .map_err(|error| error.to_string())?;

    *cache().lock().map_err(|error| error.to_string())? = Some(Cached {
        at: Instant::now(),
        usage: usage.clone(),
    });
    Ok(usage)
}
