use super::Detector;
use sysinfo::System;

pub struct TeamsDetector;

impl Detector for TeamsDetector {
    fn id(&self) -> &str {
        "teams"
    }
    fn detect(&self, _system: &System) -> bool {
        // Teams is matched from a visible window title, not a background process.
        false
    }
}
