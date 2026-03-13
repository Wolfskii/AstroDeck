use super::Detector;
use sysinfo::System;

pub struct VscodeDetector;

impl Detector for VscodeDetector {
    fn id(&self) -> &str {
        "vscode"
    }

    fn detect(&self, system: &System) -> bool {
        system.processes().values().any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            name.contains("code") && !name.contains("codec")
        })
    }
}
