use serde::Serialize;
use std::path::{Path, PathBuf};

const GITHUB_REPO: &str = "Wolfskii/AstroDeck";
const USER_AGENT: &str = "AstroDeck-Updater";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_name: Option<String>,
    pub release_notes: Option<String>,
    pub release_url: Option<String>,
    pub download_url: Option<String>,
    pub installer_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    body: Option<String>,
    html_url: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

use serde::Deserialize;

pub fn current_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn get_app_version() -> String {
    current_app_version()
}

fn empty_update_info() -> AppUpdateInfo {
    AppUpdateInfo {
        available: false,
        current_version: current_app_version(),
        latest_version: None,
        release_name: None,
        release_notes: None,
        release_url: None,
        download_url: None,
        installer_name: None,
    }
}

#[tauri::command]
pub fn check_for_app_update() -> Result<AppUpdateInfo, String> {
    if cfg!(debug_assertions) {
        return Ok(empty_update_info());
    }

    let current = current_app_version();
    let client = http_client()?;
    let releases = fetch_recent_releases(&client)?;
    let candidate = pick_release_candidate(&releases, &current);

    let Some(release) = candidate else {
        return Ok(AppUpdateInfo {
            available: false,
            current_version: current,
            latest_version: None,
            release_name: None,
            release_notes: None,
            release_url: None,
            download_url: None,
            installer_name: None,
        });
    };

    let (installer_name, download_url) = match pick_installer_asset(&release.assets) {
        Some(asset) => (Some(asset.name.clone()), Some(asset.browser_download_url.clone())),
        None => (None, None),
    };

    Ok(AppUpdateInfo {
        available: download_url.is_some(),
        current_version: current,
        latest_version: Some(normalize_tag_version(&release.tag_name)),
        release_name: Some(release.name.clone()),
        release_notes: release.body.clone(),
        release_url: Some(release.html_url.clone()),
        download_url,
        installer_name,
    })
}

#[tauri::command]
pub fn download_and_install_update(download_url: String, app: tauri::AppHandle) -> Result<(), String> {
    let client = http_client()?;
    let installer_path = download_installer(&client, &download_url)?;
    launch_installer(&installer_path)?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(750));
        app.exit(0);
    });
    Ok(())
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())
}

fn fetch_recent_releases(client: &reqwest::blocking::Client) -> Result<Vec<GitHubRelease>, String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=12");
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "GitHub releases request failed with status {}",
            response.status()
        ));
    }
    response.json::<Vec<GitHubRelease>>().map_err(|e| e.to_string())
}

fn pick_release_candidate<'a>(
    releases: &'a [GitHubRelease],
    current_version: &str,
) -> Option<&'a GitHubRelease> {
    let mut stable: Option<&GitHubRelease> = None;
    let mut prerelease: Option<&GitHubRelease> = None;

    for release in releases.iter().filter(|r| !r.draft) {
        if !is_version_newer(&release.tag_name, current_version) {
            continue;
        }
        if !release.prerelease {
            stable = Some(release);
            break;
        }
        if prerelease.is_none() {
            prerelease = Some(release);
        }
    }

    stable.or(prerelease)
}

fn pick_installer_asset(assets: &[GitHubReleaseAsset]) -> Option<&GitHubReleaseAsset> {
    let ranked = installer_asset_rank();
    for pattern in ranked {
        if let Some(asset) = assets.iter().find(|asset| pattern(&asset.name)) {
            return Some(asset);
        }
    }
    None
}

fn installer_asset_rank() -> Vec<fn(&str) -> bool> {
    #[cfg(target_os = "windows")]
    {
        vec![
            |name| {
                let lower = name.to_ascii_lowercase();
                lower.starts_with("installer-") && lower.ends_with("-setup.exe")
            },
            |name| {
                let lower = name.to_ascii_lowercase();
                lower.starts_with("installer-") && lower.ends_with(".msi")
            },
        ]
    }
    #[cfg(target_os = "linux")]
    {
        vec![
            |name| {
                let lower = name.to_ascii_lowercase();
                lower.starts_with("installer-") && lower.ends_with(".deb")
            },
            |name| {
                let lower = name.to_ascii_lowercase();
                lower.starts_with("portable-") && lower.ends_with(".appimage")
            },
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("installer-") && lower.ends_with(".dmg")
        }]
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        vec![]
    }
}

fn normalize_tag_version(tag: &str) -> String {
    tag.trim().trim_start_matches('v').to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppVersion {
    major: u32,
    minor: u32,
    patch: u32,
    /// Local/dev build (`-dev`, `-1`, `-2`). Channel releases have `None`.
    local: Option<u32>,
}

impl AppVersion {
    fn core(self) -> (u32, u32, u32) {
        (self.major, self.minor, self.patch)
    }
}

fn parse_app_version(version: &str) -> Option<AppVersion> {
    let version = version.trim().trim_start_matches('v');
    let (core, suffix) = match version.split_once('-') {
        Some((core, suffix)) => (core, Some(suffix)),
        None => (version, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    let local = match suffix {
        None | Some("") => None,
        Some("dev") => Some(0),
        Some(rest) => {
            let number = rest.split(|c: char| !c.is_ascii_digit()).next().unwrap_or("");
            Some(number.parse().unwrap_or(0))
        }
    };
    Some(AppVersion {
        major,
        minor,
        patch,
        local,
    })
}

/// GitHub is newer only when major.minor.patch is strictly higher.
/// Same x.y.z: a local `-1` / `-2` / `-dev` build is assumed to have newer code.
fn is_version_newer(candidate: &str, current: &str) -> bool {
    match (parse_app_version(candidate), parse_app_version(current)) {
        (Some(candidate), Some(current)) => candidate.core() > current.core(),
        _ => false,
    }
}

fn download_installer(
    client: &reqwest::blocking::Client,
    download_url: &str,
) -> Result<PathBuf, String> {
    let response = client
        .get(download_url)
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status {}",
            response.status()
        ));
    }

    let file_name = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "AstroDeck-update.bin".to_string());

    let target_dir = std::env::temp_dir().join("AstroDeck-update");
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    let target_path = target_dir.join(file_name);

    let bytes = response.bytes().map_err(|e| format!("Failed to read installer: {e}"))?;
    std::fs::write(&target_path, bytes).map_err(|e| format!("Failed to save installer: {e}"))?;

    Ok(target_path)
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Program Files installs require elevation. `Command::spawn` cannot trigger UAC
/// (Windows error 740), so we ShellExecute with the `runas` verb.
#[cfg(target_os = "windows")]
fn launch_elevated(file: &str, params: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HWND};
    use windows::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let mut verb = to_wide("runas");
    let mut file_w = to_wide(file);
    let mut params_w = to_wide(params);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        hwnd: HWND::default(),
        lpVerb: PCWSTR(verb.as_mut_ptr()),
        lpFile: PCWSTR(file_w.as_mut_ptr()),
        lpParameters: PCWSTR(params_w.as_mut_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    unsafe { ShellExecuteExW(&mut info) }.map_err(|err: windows::core::Error| {
        if err.code() == ERROR_CANCELLED.to_hresult() {
            "Update cancelled".to_string()
        } else {
            format!("Failed to launch installer: {err}")
        }
    })?;

    if !info.hProcess.is_invalid() {
        let _ = unsafe { CloseHandle(info.hProcess) };
    }
    Ok(())
}

fn launch_installer(path: &Path) -> Result<(), String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "Installer path is not valid UTF-8".to_string())?;

    #[cfg(target_os = "windows")]
    {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if extension == "msi" {
            let params = format!("/i \"{path_str}\" /passive /norestart");
            launch_elevated("msiexec", &params)?;
            return Ok(());
        }

        launch_elevated(path_str, "/P /UPDATE")?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let lower = path_str.to_ascii_lowercase();
        if lower.ends_with(".deb") {
            std::process::Command::new("pkexec")
                .args(["dpkg", "-i", path_str])
                .spawn()
                .map_err(|e| format!("Failed to launch package installer: {e}"))?;
            return Ok(());
        }
        if lower.ends_with(".appimage") {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            std::process::Command::new(path_str)
                .spawn()
                .map_err(|e| format!("Failed to launch AppImage: {e}"))?;
            return Ok(());
        }
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path_str)
            .spawn()
            .map_err(|e| format!("Failed to open installer: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        return Err("No supported Linux installer type found in the release asset".to_string());
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = path_str;
        Err("Automatic updates are not supported on this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            name: tag.to_string(),
            body: None,
            html_url: "https://example.invalid".to_string(),
            prerelease,
            draft: false,
            assets: vec![],
        }
    }

    #[test]
    fn compares_semver_versions() {
        assert!(is_version_newer("0.1.2", "0.1.1"));
        assert!(!is_version_newer("0.1.1", "0.1.2"));
        assert!(is_version_newer("0.1.5", "0.1.4-dev"));
        assert!(is_version_newer("0.1.6-dev", "0.1.5-2"));
        assert!(is_version_newer("v0.2.0", "0.1.5-2"));
        assert!(!is_version_newer("0.1.4", "0.1.4-dev"));
        assert!(!is_version_newer("0.1.5", "0.1.5-1"));
        assert!(!is_version_newer("0.1.5-dev", "0.1.5-2"));
        assert!(!is_version_newer("v0.1.5", "0.1.5-2"));
        assert!(!is_version_newer("0.1.5-1", "0.1.5-2"));
    }

    #[test]
    fn parses_local_dev_suffix() {
        assert_eq!(parse_app_version("0.1.5-dev").unwrap().local, Some(0));
        assert_eq!(parse_app_version("0.1.5-2").unwrap().local, Some(2));
        assert_eq!(parse_app_version("v0.1.5").unwrap().local, None);
    }

    #[test]
    fn local_build_ignores_same_triple_github_tags() {
        let releases = vec![release("v0.1.5-dev", true), release("v0.1.5", false)];
        assert!(pick_release_candidate(&releases, "0.1.5-2").is_none());
        assert_eq!(
            pick_release_candidate(&releases, "0.1.4-1").map(|r| r.tag_name.as_str()),
            Some("v0.1.5")
        );
    }

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalize_tag_version("v0.1.2-dev"), "0.1.2-dev");
        assert_eq!(normalize_tag_version("v1.0.0"), "1.0.0");
    }
}
