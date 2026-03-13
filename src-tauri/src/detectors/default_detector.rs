use super::Detector;
use sysinfo::System;

pub struct DefaultDetector;

impl Detector for DefaultDetector {
    fn id(&self) -> &str {
        "default"
    }

    fn detect(&self, _system: &System) -> bool {
        true
    }
}
