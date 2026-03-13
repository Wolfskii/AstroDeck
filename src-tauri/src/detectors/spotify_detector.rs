use super::Detector;
use sysinfo::System;

pub struct SpotifyDetector;

impl Detector for SpotifyDetector {
    fn id(&self) -> &str {
        "spotify"
    }

    fn detect(&self, system: &System) -> bool {
        system.processes().values().any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("spotify")
        })
    }
}
