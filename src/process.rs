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
        // Refresh processes to get the latest list
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let target = executable_name.to_lowercase();

        for process in self.sys.processes().values() {
            if let Some(name) = process.name().to_str()
                && name.to_lowercase() == target
            {
                return true;
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
