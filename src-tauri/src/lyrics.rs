use reqwest::blocking::Client;
use serde::Deserialize;
use base64::Engine;
use std::io::Read;

const LRCLIB_SEARCH: &str = "https://lrclib.net/api/search";
const NETEASE_SEARCH: &str = "https://music.163.com/api/search/get";
const NETEASE_LYRIC: &str = "https://music.163.com/api/song/lyric/v1";
const MUSIXMATCH_TOKEN: &str =
    "https://apic-desktop.musixmatch.com/ws/1.1/token.get";
const MUSIXMATCH_SUBTITLES: &str =
    "https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get";
const KUGOU_SEARCH: &str = "https://mobiles.kugou.com/api/v3/search/song";
const KUGOU_CANDIDATES: &str = "https://lyrics.kugou.com/search";
const KUGOU_DOWNLOAD: &str = "https://lyrics.kugou.com/download";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub start_time_ms: u64,
    pub words: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsDocument {
    pub track_id: String,
    pub provider: String,
    pub sync_type: String,
    pub available: bool,
    pub lines: Vec<LyricLine>,
}

#[derive(Debug, Deserialize)]
struct LrcLibResult {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    /// Seconds.
    duration: Option<f64>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    _plain_lyrics: Option<String>,
}

/// Longest gap between the wanted track length and a result's length to still count as the same recording.
const MATCH_DURATION_TOLERANCE_MS: u64 = 4_000;

/// Lower-case letters and digits only, without bracketed notes, "feat." credits or " - Remastered" tails,
/// so "Song (Remastered) - 2011" and "song" compare equal.
fn normalize_for_match(text: &str) -> String {
    let mut plain = String::new();
    let mut depth = 0u32;
    for ch in text.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => plain.push(ch),
            _ => {}
        }
    }
    let mut plain = plain.to_lowercase();
    for cut in [" - ", " – ", " feat.", " feat ", " ft.", " ft ", " featuring "] {
        if let Some(index) = plain.find(cut) {
            plain.truncate(index);
        }
    }
    plain
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn titles_match(wanted: &str, found: &str) -> bool {
    let (wanted, found) = (normalize_for_match(wanted), normalize_for_match(found));
    if wanted.is_empty() || found.is_empty() {
        return false;
    }
    if wanted == found {
        return true;
    }
    let (short, long) = if wanted.len() <= found.len() {
        (&wanted, &found)
    } else {
        (&found, &wanted)
    };
    short.chars().count() >= 6 && long.contains(short.as_str())
}

/// The wanted artist may be one of several credited names in the result, or the other way round.
fn artists_match(wanted: &str, found: &str) -> bool {
    let (wanted, found) = (normalize_for_match(wanted), normalize_for_match(found));
    if wanted.is_empty() || found.is_empty() {
        return false;
    }
    wanted == found
        || (wanted.chars().count() >= 3 && found.contains(wanted.as_str()))
        || (found.chars().count() >= 3 && wanted.contains(found.as_str()))
}

fn durations_match(wanted_ms: Option<u64>, found_ms: Option<u64>) -> bool {
    match (wanted_ms, found_ms) {
        (Some(wanted), Some(found)) => wanted.abs_diff(found) <= MATCH_DURATION_TOLERANCE_MS,
        // Without a length on one side there is nothing to compare, so title and artist decide.
        _ => true,
    }
}

/// Whether a search result is the track we asked for.
fn is_same_track(
    title: &str,
    artist: &str,
    duration_ms: Option<u64>,
    found_title: &str,
    found_artist: &str,
    found_duration_ms: Option<u64>,
) -> bool {
    titles_match(title, found_title)
        && artists_match(artist, found_artist)
        && durations_match(duration_ms, found_duration_ms)
}

#[derive(Debug, Deserialize)]
struct NetEaseSearchResponse {
    result: Option<NetEaseSearchResult>,
}

#[derive(Debug, Deserialize)]
struct NetEaseSearchResult {
    songs: Option<Vec<NetEaseSong>>,
}

#[derive(Debug, Deserialize)]
struct NetEaseSong {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    artists: Vec<NetEaseArtist>,
    /// Milliseconds.
    duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct NetEaseArtist {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct NetEaseLyricsResponse {
    lrc: Option<NetEaseLyricsBody>,
    yrc: Option<NetEaseLyricsBody>,
}

#[derive(Debug, Deserialize)]
struct NetEaseLyricsBody {
    lyric: Option<String>,
}

pub fn fetch(
    order: &[String],
    track_id: &str,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_ms: Option<u64>,
) -> Result<LyricsDocument, String> {
    let client = Client::builder()
        .user_agent("AstroDeck lyrics")
        .build()
        .map_err(|error| error.to_string())?;
    for provider in order {
        let result = match provider.as_str() {
            "lrclib" => fetch_lrclib(&client, track_id, title, artist, album, duration_ms),
            "musixmatch" => fetch_musixmatch(&client, track_id, title, artist, album, duration_ms),
            "kugou" => fetch_kugou(&client, track_id, title, artist, duration_ms),
            "netease" => fetch_netease(&client, track_id, title, artist, duration_ms),
            _ => Err(format!("Unknown lyrics provider: {provider}")),
        };
        if let Ok(document) = result {
            if document.available {
                return Ok(document);
            }
        }
    }
    Ok(LyricsDocument {
        track_id: track_id.to_string(),
        provider: "none".to_string(),
        sync_type: "UNSYNCED".to_string(),
        available: false,
        lines: Vec::new(),
    })
}

fn fetch_musixmatch(
    client: &Client,
    track_id: &str,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_ms: Option<u64>,
) -> Result<LyricsDocument, String> {
    let token_response: serde_json::Value = client
        .get(MUSIXMATCH_TOKEN)
        .query(&[("format", "json"), ("app_id", "web-desktop-app-v1.0")])
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;
    let token = token_response
        .pointer("/message/body/user_token")
        .and_then(serde_json::Value::as_str)
        .ok_or("Musixmatch did not return a token")?;
    let mut params = vec![
        ("format", "json".to_string()),
        ("namespace", "lyrics_richsynched".to_string()),
        ("subtitle_format", "mxm".to_string()),
        ("optional_calls", "track.richsync".to_string()),
        ("app_id", "web-desktop-app-v1.0".to_string()),
        ("usertoken", token.to_string()),
        ("q_track", title.to_string()),
        ("q_artist", artist.to_string()),
    ];
    if let Some(album) = album.filter(|album| !album.is_empty()) {
        params.push(("q_album", album.to_string()));
    }
    if let Some(duration_ms) = duration_ms {
        params.push(("q_duration", (duration_ms / 1000).to_string()));
    }
    let response: serde_json::Value = client
        .get(MUSIXMATCH_SUBTITLES)
        .query(&params)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;
    // Musixmatch's matcher can answer with a different song, so check what it actually found.
    let matched = response.pointer("/message/body/macro_calls/matcher.track.get/message/body/track");
    let matched_ok = matched.is_some_and(|track| {
        let text = |key: &str| track.get(key).and_then(serde_json::Value::as_str).unwrap_or_default();
        let seconds = track.get("track_length").and_then(serde_json::Value::as_u64);
        is_same_track(
            title,
            artist,
            duration_ms,
            text("track_name"),
            text("artist_name"),
            seconds.map(|seconds| seconds * 1000),
        )
    });
    if !matched_ok {
        return Err("Musixmatch has no close match".to_string());
    }
    let richsync = response
        .pointer("/message/body/macro_calls/track.richsync.get/message/body/richsync/richsync_body")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_musixmatch_richsync);
    let lines = richsync.or_else(|| {
        response
            .pointer(
                "/message/body/macro_calls/track.subtitles.get/message/body/subtitle_list/0/subtitle/subtitle_body",
            )
            .and_then(serde_json::Value::as_str)
            .and_then(parse_musixmatch_subtitles)
    });
    let lines = lines.filter(|lines| !lines.is_empty()).ok_or("Musixmatch returned no lyrics")?;
    Ok(LyricsDocument {
        track_id: track_id.to_string(),
        provider: "musixmatch".to_string(),
        sync_type: "LINE_SYNCED".to_string(),
        available: true,
        lines,
    })
}

fn parse_musixmatch_richsync(raw: &str) -> Option<Vec<LyricLine>> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(raw).ok()?;
    let lines = rows
        .into_iter()
        .filter_map(|row| {
            let start = row.get("ts")?.as_f64()?;
            let words = row.get("x")?.as_str()?.trim().to_string();
            (!words.is_empty()).then_some(LyricLine {
                start_time_ms: (start * 1000.0) as u64,
                words,
            })
        })
        .collect::<Vec<_>>();
    (!lines.is_empty()).then_some(lines)
}

fn parse_musixmatch_subtitles(raw: &str) -> Option<Vec<LyricLine>> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(raw).ok()?;
    let lines = rows
        .into_iter()
        .filter_map(|row| {
            let words = row.get("text")?.as_str()?.trim().to_string();
            let start = row.pointer("/time/total")?.as_f64()?;
            (!words.is_empty()).then_some(LyricLine {
                start_time_ms: (start * 1000.0) as u64,
                words,
            })
        })
        .collect::<Vec<_>>();
    (!lines.is_empty()).then_some(lines)
}

fn fetch_kugou(
    client: &Client,
    track_id: &str,
    title: &str,
    artist: &str,
    duration_ms: Option<u64>,
) -> Result<LyricsDocument, String> {
    let query = format!("{artist} {title}");
    let search: serde_json::Value = client
        .get(KUGOU_SEARCH)
        .query(&[
            ("format", "json"),
            ("keyword", query.as_str()),
            ("page", "1"),
            ("pagesize", "5"),
            ("showtype", "1"),
        ])
        .send()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;
    let song = search
        .pointer("/data/info")
        .and_then(serde_json::Value::as_array)
        .and_then(|songs| {
            songs.iter().find(|song| {
                let text = |key: &str| song.get(key).and_then(serde_json::Value::as_str).unwrap_or_default();
                let seconds = song.get("duration").and_then(serde_json::Value::as_u64);
                is_same_track(
                    title,
                    artist,
                    duration_ms,
                    text("songname"),
                    text("singername"),
                    seconds.map(|seconds| seconds * 1000),
                )
            })
        })
        .ok_or("Kugou has no close match")?;
    let hash = song.get("hash").and_then(serde_json::Value::as_str).ok_or("Kugou result has no hash")?;
    let duration = duration_ms.unwrap_or_default().to_string();
    let candidates: serde_json::Value = client
        .get(KUGOU_CANDIDATES)
        .query(&[
            ("ver", "1"),
            ("man", "yes"),
            ("client", "mobi"),
            ("hash", hash),
            ("duration", duration.as_str()),
        ])
        .send()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;
    let candidate = candidates
        .pointer("/candidates/0")
        .ok_or("Kugou returned no lyric candidates")?;
    let id = candidate
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or("Kugou candidate has no id")?;
    let access_key = candidate
        .get("accesskey")
        .and_then(serde_json::Value::as_str)
        .ok_or("Kugou candidate has no access key")?;
    let download: serde_json::Value = client
        .get(KUGOU_DOWNLOAD)
        .query(&[
            ("ver", "1"),
            ("client", "pc"),
            ("id", id),
            ("accesskey", access_key),
            ("fmt", "krc"),
            ("charset", "utf8"),
        ])
        .send()
        .map_err(|error| error.to_string())?
        .json()
        .map_err(|error| error.to_string())?;
    let encoded = download
        .get("content")
        .and_then(serde_json::Value::as_str)
        .filter(|content| !content.is_empty())
        .ok_or("Kugou returned empty lyrics")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| error.to_string())?;
    let cipher = [
        0x40, 0x47, 0x61, 0x77, 0x5e, 0x32, 0x74, 0x47, 0x51, 0x36, 0x31, 0x2d, 0xce, 0xd2,
        0x6e, 0x69,
    ];
    let body = decoded.get(4..).ok_or("Kugou lyrics header is invalid")?;
    let unmasked = body
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ cipher[index % cipher.len()])
        .collect::<Vec<_>>();
    let mut inflated = String::new();
    flate2::read::ZlibDecoder::new(unmasked.as_slice())
        .read_to_string(&mut inflated)
        .map_err(|error| error.to_string())?;
    let lines = parse_kugou(&inflated);
    if lines.is_empty() {
        return Err("Kugou returned no synced lyrics".to_string());
    }
    Ok(LyricsDocument {
        track_id: track_id.to_string(),
        provider: "kugou".to_string(),
        sync_type: "LINE_SYNCED".to_string(),
        available: true,
        lines,
    })
}

fn parse_kugou(text: &str) -> Vec<LyricLine> {
    text.lines()
        .filter_map(|line| {
            let (header, rest) = line.strip_prefix('[')?.split_once(']')?;
            let (start, span) = header.split_once(',')?;
            let start = start.parse::<u64>().ok()?;
            let _span = span.parse::<u64>().ok()?;
            let words = rest
                .split('<')
                .filter_map(|part| part.split_once('>').map(|(_, text)| text))
                .collect::<String>();
            let words = if words.is_empty() {
                rest.to_string()
            } else {
                words
            };
            (!words.trim().is_empty()).then_some(LyricLine {
                start_time_ms: start,
                words: words.trim().to_string(),
            })
        })
        .collect()
}

fn fetch_lrclib(
    client: &Client,
    track_id: &str,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_ms: Option<u64>,
) -> Result<LyricsDocument, String> {
    let mut query = vec![("track_name", title), ("artist_name", artist)];
    if let Some(album) = album {
        query.push(("album_name", album));
    }
    let response = client
        .get(LRCLIB_SEARCH)
        .query(&query)
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("LRCLIB returned {}", response.status()));
    }
    let rows: Vec<LrcLibResult> = response.json().map_err(|error| error.to_string())?;
    let row = rows
        .into_iter()
        .filter(|row| {
            row.synced_lyrics.as_deref().is_some_and(|lyrics| !lyrics.trim().is_empty())
                && is_same_track(
                    title,
                    artist,
                    duration_ms,
                    row.track_name.as_deref().unwrap_or_default(),
                    row.artist_name.as_deref().unwrap_or_default(),
                    row.duration.map(|seconds| (seconds * 1000.0) as u64),
                )
        })
        .min_by_key(|row| {
            let found = row.duration.map(|seconds| (seconds * 1000.0) as u64);
            match (duration_ms, found) {
                (Some(wanted), Some(found)) => wanted.abs_diff(found),
                _ => u64::MAX,
            }
        })
        .ok_or("LRCLIB has no close match")?;
    let lines = row
        .synced_lyrics
        .as_deref()
        .map(parse_lrc)
        .filter(|lines| !lines.is_empty())
        .unwrap_or_default();
    if lines.is_empty() {
        return Err("LRCLIB returned no synced lyrics".to_string());
    }
    if let (Some(expected), Some(last)) = (duration_ms, lines.last()) {
        if last.start_time_ms > expected.saturating_add(120_000) {
            return Err("LRCLIB result duration mismatch".to_string());
        }
    }
    Ok(LyricsDocument {
        track_id: track_id.to_string(),
        provider: "lrclib".to_string(),
        sync_type: "LINE_SYNCED".to_string(),
        available: true,
        lines,
    })
}

fn fetch_netease(
    client: &Client,
    track_id: &str,
    title: &str,
    artist: &str,
    duration_ms: Option<u64>,
) -> Result<LyricsDocument, String> {
    let query = format!("{title} {artist}");
    let response = client
        .get(NETEASE_SEARCH)
        .query(&[("s", query.as_str()), ("type", "1"), ("limit", "5")])
        .header("Referer", "https://music.163.com")
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("NetEase search returned {}", response.status()));
    }
    let search: NetEaseSearchResponse = response.json().map_err(|error| error.to_string())?;
    let song_id = search
        .result
        .and_then(|result| result.songs)
        .and_then(|songs| {
            songs
                .into_iter()
                .find(|song| {
                    let artists = song
                        .artists
                        .iter()
                        .map(|artist| artist.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    is_same_track(title, artist, duration_ms, &song.name, &artists, song.duration)
                })
                .map(|song| song.id)
        })
        .ok_or("NetEase has no close match")?;
    let response = client
        .get(NETEASE_LYRIC)
        .query(&[
            ("id", song_id.to_string()),
            ("lv", "1".to_string()),
            ("kv", "1".to_string()),
            ("tv", "-1".to_string()),
        ])
        .header("Referer", "https://music.163.com")
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("NetEase lyrics returned {}", response.status()));
    }
    let lyrics: NetEaseLyricsResponse = response.json().map_err(|error| error.to_string())?;
    let raw = lyrics
        .yrc
        .and_then(|body| body.lyric)
        .or_else(|| lyrics.lrc.and_then(|body| body.lyric))
        .ok_or("NetEase returned no lyrics")?;
    let lines = parse_lrc(&raw);
    if lines.is_empty() {
        return Err("NetEase returned no synced lyrics".to_string());
    }
    if let (Some(expected), Some(last)) = (duration_ms, lines.last()) {
        if last.start_time_ms > expected.saturating_add(120_000) {
            return Err("NetEase result duration mismatch".to_string());
        }
    }
    Ok(LyricsDocument {
        track_id: track_id.to_string(),
        provider: "netease".to_string(),
        sync_type: "LINE_SYNCED".to_string(),
        available: true,
        lines,
    })
}

fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let mut lines = text
        .lines()
        .filter_map(|line| {
            let close = line.find(']')?;
            let stamp = line.get(1..close)?;
            let (minutes, seconds) = stamp.split_once(':')?;
            let minutes = minutes.parse::<u64>().ok()?;
            let (seconds, fraction) = seconds.split_once('.').unwrap_or((seconds, "0"));
            let seconds = seconds.parse::<u64>().ok()?;
            let fraction_text = fraction.chars().take(3).collect::<String>();
            let fraction = fraction_text.parse::<u64>().ok().unwrap_or(0);
            let fraction = match fraction_text.len() {
                1 => fraction * 100,
                2 => fraction * 10,
                _ => fraction,
            };
            let words = line[close + 1..].trim().to_string();
            (!words.is_empty()).then_some(LyricLine {
                start_time_ms: minutes * 60_000 + seconds * 1_000 + fraction,
                words,
            })
        })
        .collect::<Vec<_>>();
    lines.sort_by_key(|line| line.start_time_ms);
    lines
}

#[cfg(test)]
mod tests {
    use super::{is_same_track, parse_lrc};

    #[test]
    fn matches_the_same_track_despite_decorations() {
        assert!(is_same_track(
            "Sweater Weather",
            "The Neighbourhood",
            Some(240_400),
            "Sweater Weather (Remastered)",
            "The Neighbourhood",
            Some(240_465),
        ));
        assert!(is_same_track(
            "Blinding Lights - Radio Edit",
            "The Weeknd",
            Some(200_000),
            "Blinding Lights",
            "The Weeknd, Someone Else",
            None,
        ));
    }

    #[test]
    fn rejects_other_songs_and_other_versions() {
        // A different song entirely.
        assert!(!is_same_track("Sweater Weather", "The Neighbourhood", Some(240_000), "NOKIA", "Drake", None));
        // The right song, but a much shorter edit.
        assert!(!is_same_track(
            "Sweater Weather",
            "The Neighbourhood",
            Some(240_400),
            "Sweater Weather",
            "The Neighbourhood",
            Some(138_000),
        ));
        // Same title by another artist.
        assert!(!is_same_track("Home", "Edward Sharpe", Some(180_000), "Home", "Michael Buble", Some(180_000)));
    }

    #[test]
    fn parses_lrc_timestamps() {
        let lines = parse_lrc("[00:01.20]First\n[01:02.345]Second");
        assert_eq!(lines[0].start_time_ms, 1_200);
        assert_eq!(lines[1].start_time_ms, 62_345);
    }
}
