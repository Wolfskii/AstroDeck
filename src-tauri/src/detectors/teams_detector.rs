use super::Detector;
use sysinfo::System;

pub struct TeamsDetector;

impl Detector for TeamsDetector {
    fn id(&self) -> &str {
        "teams"
    }

    fn detect(&self, system: &System) -> bool {
        system.processes().values().any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("teams") || name.contains("ms-teams")
        })
    }
}
