use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::Deserialize;
use tauri::Emitter;

const FAST_TOKEN_FALLBACK: &str = "YXNkZmFzZGxmbnNkYWZoYXNkZmhrYWxm";
const MEASURE_FOR: Duration = Duration::from_secs(7);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedProgress {
    id: String,
    phase: String,
    ping_ms: Option<f64>,
    down_mbps: Option<f64>,
    up_mbps: Option<f64>,
    server: Option<String>,
    location: Option<String>,
    error: Option<String>,
}

struct Live {
    bytes: AtomicU64,
    started: Instant,
    stop: AtomicBool,
    last_emit: Mutex<Instant>,
}

fn flags() -> &'static Mutex<std::collections::HashMap<String, std::sync::Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<std::collections::HashMap<String, std::sync::Arc<AtomicBool>>>> =
        OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("AstroDeck")
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| "Couldn't start the speed test".to_string())
}

fn emit(app: &tauri::AppHandle, progress: SpeedProgress) {
    let _ = app.emit("speed-test-progress", progress);
}

fn mbps(bytes: u64, started: Instant) -> f64 {
    let seconds = started.elapsed().as_secs_f64().max(0.05);
    (bytes as f64) * 8.0 / seconds / 1_000_000.0
}

fn median(mut samples: Vec<f64>) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

fn cancelled(flag: &AtomicBool) -> bool {
    flag.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn start_speed_test(app: tauri::AppHandle, id: String, provider: String) -> Result<(), String> {
    let flag = std::sync::Arc::new(AtomicBool::new(false));
    flags().lock().unwrap().insert(id.clone(), flag.clone());
    let result = match provider.as_str() {
        "fast" => run_fast(&app, &id, &flag),
        "bredbandskollen" => run_bredbandskollen(&app, &id, &flag),
        _ => Err("Pick Cloudflare, Fast.com, or Bredbandskollen".to_string()),
    };
    flags().lock().unwrap().remove(&id);
    if cancelled(&flag) {
        return Ok(());
    }
    if let Err(message) = &result {
        emit(
            &app,
            SpeedProgress {
                id,
                phase: "error".to_string(),
                ping_ms: None,
                down_mbps: None,
                up_mbps: None,
                server: None,
                location: None,
                error: Some(message.clone()),
            },
        );
    }
    result
}

#[tauri::command]
pub fn cancel_speed_test(id: String) {
    if let Some(flag) = flags().lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
    }
}

fn run_fast(app: &tauri::AppHandle, id: &str, flag: &AtomicBool) -> Result<(), String> {
    let http = client()?;
    emit_phase(app, id, "ping", None, None, None);
    let target = fast_target(&http)?;
    emit_place(app, id, "ping", None, None, None, &target.server, &target.location);
    if !https_host(&target.url, ".nflxvideo.net") {
        return Err("Fast.com didn't respond".to_string());
    }
    let ping = measure_ping(flag, || ping_once(&http, &with_range(&target.url, 0)?))?;
    if cancelled(flag) {
        return Ok(());
    }
    emit_phase(app, id, "download", Some(ping), None, None);
    let down = transfer(app, id, flag, "download", Some(ping), None, 3, |until| {
        let response = http
            .get(with_range(&target.url, 25_000_000)?)
            .send()
            .map_err(|_| "Download test failed".to_string())?;
        read_body(response, until)
    })?;
    if cancelled(flag) {
        return Ok(());
    }
    emit_phase(app, id, "upload", Some(ping), Some(down), None);
    let payload = vec![0u8; 1_000_000];
    let up = transfer(app, id, flag, "upload", Some(ping), Some(down), 2, |_until| {
        let response = http
            .post(with_range(&target.url, 0)?)
            .body(payload.clone())
            .send()
            .map_err(|_| "Upload test failed".to_string())?;
        if !response.status().is_success() {
            return Err("Upload test failed".to_string());
        }
        Ok(payload.len() as u64)
    })?;
    emit_phase(app, id, "done", Some(ping), Some(down), Some(up));
    Ok(())
}

fn run_bredbandskollen(app: &tauri::AppHandle, id: &str, flag: &AtomicBool) -> Result<(), String> {
    let http = client()?;
    emit_phase(app, id, "ping", None, None, None);
    let settings: BbkServers = http
        .get("https://frontend.bredbandskollen.se/api/servers")
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|_| "Bredbandskollen didn't respond".to_string())?;
    let server = settings
        .servers
        .iter()
        .find(|server| {
            server.kind == "ipv4"
                && server.tlsport == 443
                && server.url.contains("anycast")
                && bbk_host(&server.url)
        })
        .or_else(|| {
            settings.servers.iter().find(|server| {
                server.kind == "ipv4" && server.tlsport == 443 && bbk_host(&server.url)
            })
        })
        .ok_or_else(|| "Bredbandskollen didn't respond".to_string())?;
    let host = server.url.clone();
    let name = server.name.clone();
    let ticket = bbk_ticket(&http, &host, &settings.hashkey)?;
    let (server_name, location) = bbk_place(&ticket, &name);
    emit_place(app, id, "ping", None, None, None, &server_name, &location);
    let ping = measure_ping(flag, || {
        let url = format!("https://{host}/pingpong/1?t={ticket}");
        ping_once(&http, &url)
    })?;
    if cancelled(flag) {
        return Ok(());
    }
    emit_phase(app, id, "download", Some(ping), None, None);
    let down = transfer(app, id, flag, "download", Some(ping), None, 3, |until| {
        let url = format!(
            "https://{host}/bigfile.bin?t={ticket}&len=8000000&id=1&b={}",
            rand_token()
        );
        let response = http
            .get(url)
            .send()
            .map_err(|_| "Download test failed".to_string())?;
        read_body(response, until)
    })?;
    if cancelled(flag) {
        return Ok(());
    }
    emit_phase(app, id, "upload", Some(ping), Some(down), None);
    let payload = vec![0u8; 1_000_000];
    let up = transfer(app, id, flag, "upload", Some(ping), Some(down), 2, |_until| {
        let url = format!(
            "https://{host}/cgi/upload.cgi?t={ticket}&id=1&b={}",
            rand_token()
        );
        let response = http
            .post(url)
            .body(payload.clone())
            .send()
            .map_err(|_| "Upload test failed".to_string())?;
        if !response.status().is_success() {
            return Err("Upload test failed".to_string());
        }
        Ok(payload.len() as u64)
    })?;
    emit_phase(app, id, "done", Some(ping), Some(down), Some(up));
    Ok(())
}

fn emit_phase(
    app: &tauri::AppHandle,
    id: &str,
    phase: &str,
    ping_ms: Option<f64>,
    down_mbps: Option<f64>,
    up_mbps: Option<f64>,
) {
    emit(
        app,
        SpeedProgress {
            id: id.to_string(),
            phase: phase.to_string(),
            ping_ms,
            down_mbps,
            up_mbps,
            server: None,
            location: None,
            error: None,
        },
    );
}

fn emit_place(
    app: &tauri::AppHandle,
    id: &str,
    phase: &str,
    ping_ms: Option<f64>,
    down_mbps: Option<f64>,
    up_mbps: Option<f64>,
    server: &str,
    location: &str,
) {
    emit(
        app,
        SpeedProgress {
            id: id.to_string(),
            phase: phase.to_string(),
            ping_ms,
            down_mbps,
            up_mbps,
            server: Some(server.to_string()).filter(|value| !value.is_empty()),
            location: Some(location.to_string()).filter(|value| !value.is_empty()),
            error: None,
        },
    );
}

fn measure_ping(flag: &AtomicBool, mut once: impl FnMut() -> Result<f64, String>) -> Result<f64, String> {
    let mut samples = Vec::new();
    for _ in 0..6 {
        if cancelled(flag) {
            return Ok(0.0);
        }
        samples.push(once()?);
    }
    Ok(median(samples))
}

fn ping_once(http: &Client, url: &str) -> Result<f64, String> {
    let started = Instant::now();
    let response = http.get(url).send().map_err(|_| "Ping test failed".to_string())?;
    if !response.status().is_success() {
        return Err("Ping test failed".to_string());
    }
    let _ = response.bytes();
    Ok(started.elapsed().as_secs_f64() * 1000.0)
}

fn transfer(
    app: &tauri::AppHandle,
    id: &str,
    flag: &AtomicBool,
    phase: &str,
    ping: Option<f64>,
    other: Option<f64>,
    workers: usize,
    transfer_once: impl Fn(Instant) -> Result<u64, String> + Sync,
) -> Result<f64, String> {
    let live = Live {
        bytes: AtomicU64::new(0),
        started: Instant::now(),
        stop: AtomicBool::new(false),
        last_emit: Mutex::new(Instant::now() - Duration::from_secs(1)),
    };
    let error = Mutex::new(None::<String>);
    let until = live.started + MEASURE_FOR;
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                if cancelled(flag) || live.stop.load(Ordering::Relaxed) || Instant::now() >= until {
                    return;
                }
                match transfer_once(until) {
                    Ok(bytes) => publish(app, id, phase, ping, other, &live, bytes),
                    Err(message) => {
                        let mut slot = error.lock().unwrap();
                        if slot.is_none() {
                            *slot = Some(message);
                        }
                        drop(slot);
                        live.stop.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            });
        }
    });
    if let Some(message) = error.lock().unwrap().clone() {
        if !cancelled(flag) {
            return Err(message);
        }
    }
    Ok(mbps(live.bytes.load(Ordering::Relaxed), live.started))
}

fn publish(
    app: &tauri::AppHandle,
    id: &str,
    phase: &str,
    ping: Option<f64>,
    other: Option<f64>,
    live: &Live,
    bytes: u64,
) {
    live.bytes.fetch_add(bytes, Ordering::Relaxed);
    let mut last = live.last_emit.lock().unwrap();
    if last.elapsed() < Duration::from_millis(180) {
        return;
    }
    *last = Instant::now();
    drop(last);
    let rate = mbps(live.bytes.load(Ordering::Relaxed), live.started);
    let (down, up) = if phase == "upload" {
        (other, Some(rate))
    } else {
        (Some(rate), None)
    };
    emit_phase(app, id, phase, ping, down, up);
}

fn read_body(mut response: reqwest::blocking::Response, until: Instant) -> Result<u64, String> {
    if !response.status().is_success() {
        return Err("Download test failed".to_string());
    }
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    if Instant::now() >= until {
        return Ok(0);
    }
    loop {
        match response.read(&mut buf) {
            Ok(0) => return Ok(total),
            Ok(read) => {
                total += read as u64;
                if Instant::now() >= until {
                    return Ok(total);
                }
            }
            Err(_) => {
                if total > 0 {
                    return Ok(total);
                }
                return Err("Download test failed".to_string());
            }
        }
    }
}

struct FastPick {
    url: String,
    server: String,
    location: String,
}

fn fast_target(http: &Client) -> Result<FastPick, String> {
    let token = fast_token(http);
    let url = format!(
        "https://api.fast.com/netflix/speedtest/v2?https=true&token={token}&urlCount=1"
    );
    let response: FastResponse = http
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|_| "Fast.com didn't respond".to_string())?;
    let target = response
        .targets
        .into_iter()
        .next()
        .ok_or_else(|| "Fast.com didn't respond".to_string())?;
    let server = fast_server_name(&target.url);
    let location = match target.location {
        Some(place) => match (place.city, place.country) {
            (Some(city), Some(country)) if !city.is_empty() && !country.is_empty() => {
                format!("{city}, {country}")
            }
            (Some(city), _) if !city.is_empty() => city,
            _ => String::new(),
        },
        None => String::new(),
    };
    Ok(FastPick {
        url: target.url,
        server,
        location,
    })
}

fn fast_server_name(url: &str) -> String {
    let host = reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_default();
    host.split('.')
        .next()
        .unwrap_or(&host)
        .split('-')
        .find(|part| {
            part.len() >= 5
                && part.chars().take(3).all(|ch| ch.is_ascii_alphabetic())
                && part.chars().skip(3).all(|ch| ch.is_ascii_digit())
        })
        .unwrap_or("Fast.com")
        .to_string()
}

fn bbk_place(ticket: &str, fallback_name: &str) -> (String, String) {
    let site: String = ticket
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect();
    let city = match site.as_str() {
        "sth" => "Stockholm",
        "gbg" => "Gothenburg",
        "mmo" => "Malmö",
        "osl" => "Oslo",
        "svl" => "Sundsvall",
        "ume" => "Umeå",
        "htz" => "Helsinki",
        _ => "",
    };
    let location = if city.is_empty() {
        fallback_name.to_string()
    } else {
        city.to_string()
    };
    (site, location)
}

fn fast_token(http: &Client) -> String {
    let page = http.get("https://fast.com/").send().ok();
    let Some(page) = page else {
        return FAST_TOKEN_FALLBACK.to_string();
    };
    let Ok(html) = page.text() else {
        return FAST_TOKEN_FALLBACK.to_string();
    };
    let Some(src) = html.split("src=\"").find_map(|part| {
        let candidate = part.split('"').next().unwrap_or("");
        candidate.starts_with("/app-").then(|| candidate.to_string())
    }) else {
        return FAST_TOKEN_FALLBACK.to_string();
    };
    let script = http
        .get(format!("https://fast.com{src}"))
        .send()
        .and_then(|response| response.text())
        .unwrap_or_default();
    script
        .split("token:\"")
        .nth(1)
        .and_then(|part| part.split('"').next())
        .filter(|token| !token.is_empty())
        .unwrap_or(FAST_TOKEN_FALLBACK)
        .to_string()
}

fn with_range(url: &str, end: u64) -> Result<String, String> {
    let (path, query) = url.split_once('?').ok_or("Fast.com didn't respond")?;
    Ok(format!("{path}/range/0-{end}?{query}"))
}

fn https_host(url: &str, suffix: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .filter(|parsed| parsed.scheme() == "https")
        .and_then(|parsed| parsed.host_str().map(|host| host.ends_with(suffix)))
        .unwrap_or(false)
}

fn bbk_host(host: &str) -> bool {
    host.ends_with(".mserver.bredbandskollen.se")
        && host.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
}

fn bbk_ticket(http: &Client, host: &str, key: &str) -> Result<String, String> {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("key", key)
        .append_pair("host", "frontend.bredbandskollen.se")
        .finish();
    let response: BbkTicket = http
        .get(format!("https://{host}/ticket?{query}"))
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|_| "Bredbandskollen didn't respond".to_string())?;
    if response.ticket.is_empty() {
        return Err("Bredbandskollen didn't respond".to_string());
    }
    Ok(response.ticket)
}

fn rand_token() -> u64 {
    Instant::now().elapsed().as_nanos() as u64
}

#[derive(Deserialize)]
struct FastResponse {
    targets: Vec<FastTarget>,
}

#[derive(Deserialize)]
struct FastTarget {
    url: String,
    location: Option<FastLocation>,
}

#[derive(Deserialize)]
struct FastLocation {
    city: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct BbkServers {
    hashkey: String,
    servers: Vec<BbkServer>,
}

#[derive(Deserialize)]
struct BbkServer {
    url: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    tlsport: u16,
}

#[derive(Deserialize)]
struct BbkTicket {
    ticket: String,
}
