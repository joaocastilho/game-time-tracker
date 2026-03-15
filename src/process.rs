use sysinfo::System;

pub struct ProcessMonitor {
    sys: System,
}

/// Strip a `.exe` suffix (case-insensitive) from a process name so that
/// `"firefox"` and `"firefox.exe"` are treated as the same executable.
fn strip_exe(name: &str) -> &str {
    // Input is expected to be already lowercased.
    name.strip_suffix(".exe").unwrap_or(name)
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self { sys: System::new() }
    }

    pub fn is_running(&mut self, executable_name: &str) -> bool {
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let target_lower = executable_name.to_lowercase();
        let target = strip_exe(&target_lower);

        for process in self.sys.processes().values() {
            let name_lower = process.name().to_str().map(|s| s.to_lowercase());
            if let Some(name_lower) = name_lower {
                if strip_exe(&name_lower) == target {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_exe_removes_extension() {
        assert_eq!(strip_exe("firefox.exe"), "firefox");
        assert_eq!(strip_exe("firefox"), "firefox");
        assert_eq!(strip_exe("some.game.exe"), "some.game");
        assert_eq!(strip_exe("noextension"), "noextension");
        assert_eq!(strip_exe(""), "");
    }

    #[test]
    fn test_process_monitor_creation() {
        let mut monitor = ProcessMonitor::new();
        assert!(!monitor.is_running("nonexistent_process_12345.exe"));
    }

    #[test]
    fn test_process_monitor_nonexistent() {
        let mut monitor = ProcessMonitor::new();
        let result = monitor.is_running("this_process_definitely_does_not_exist_12345.exe");
        assert!(!result);
    }

    #[test]
    fn test_process_monitor_case_insensitive() {
        let mut monitor = ProcessMonitor::new();
        let result1 = monitor.is_running("EXPLORER.EXE");
        let result2 = monitor.is_running("explorer.exe");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_is_running_without_exe_suffix() {
        // explorer.exe always runs on Windows; verify that omitting .exe still matches.
        let mut monitor = ProcessMonitor::new();
        let with_ext = monitor.is_running("explorer.exe");
        let without_ext = monitor.is_running("explorer");
        assert_eq!(
            with_ext, without_ext,
            "is_running should match regardless of .exe suffix"
        );
    }
}
