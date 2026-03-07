use sysinfo::System;

pub struct ProcessMonitor {
    sys: System,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn is_running(&mut self, executable_name: &str) -> bool {
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let target = executable_name.to_lowercase();

        for process in self.sys.processes().values() {
            if let Some(name) = process.name().to_str() {
                if name.to_lowercase() == target {
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
}
