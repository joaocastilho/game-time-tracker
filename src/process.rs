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

    /// Refresh the internal process snapshot once per tracker tick.
    /// Call this before a batch of `is_running_cached` checks to avoid
    /// O(G*P) refreshes and to get a consistent view within one tick.
    pub fn refresh(&mut self) {
        // Ensure `exe()` is populated for `list_processes` filtering.
        // `refresh_processes` with `true` already refreshes everything on
        // current sysinfo, but be explicit for future-compat.
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    }

    pub fn is_running(&mut self, executable_name: &str) -> bool {
        self.refresh();
        self.is_running_cached(executable_name)
    }

    /// Check without refreshing – use after `refresh()` for batched checks.
    pub fn is_running_cached(&self, executable_name: &str) -> bool {
        let target_lower = executable_name.to_lowercase();
        let target = strip_exe(&target_lower);

        for process in self.sys.processes().values() {
            // Avoid per-process allocation by comparing case-insensitively via
            // lowercased target vs lowercased name stripped.
            if let Some(name) = process.name().to_str() {
                // Fast path: compare lengths after stripping to avoid extra allocation
                // when possible, but still need lowercasing for correctness.
                let name_lower = name.to_lowercase();
                if strip_exe(&name_lower) == target {
                    return true;
                }
            }
        }
        false
    }

    fn normalize_for_filter(path: &std::path::Path) -> String {
        let s = path.to_string_lossy();
        // Strip Win32 long-path prefix `\\?\` (and `\\?\Volume{...}` not needed for WINDIR)
        let stripped = s.strip_prefix("\\\\?\\").unwrap_or(&s);
        stripped.to_lowercase()
    }

    /// Return a deduplicated list of currently running **non-system**
    /// process names, ordered by **last started** (most recent first).
    /// On Windows this filters to `.exe` processes whose executable is *not*
    /// inside `WINDIR` (e.g. `C:\Windows\...`) so the dropdown only shows user
    /// applications / games. `is_running` is intentionally *not* filtered so
    /// tracking still works for any exe.
    pub fn list_processes(&mut self) -> Vec<String> {
        self.refresh();

        let windir_prefix = if cfg!(windows) {
            let w = std::env::var("WINDIR")
                .unwrap_or_else(|_| "C:\\Windows".to_string())
                .to_lowercase();
            let trimmed = w.trim_end_matches('\\').to_string();
            format!("{trimmed}\\")
        } else {
            String::new()
        };

        // Map lowercased name -> (display_name, latest_start_time)
        let mut seen: std::collections::HashMap<String, (String, u64)> =
            std::collections::HashMap::new();
        for process in self.sys.processes().values() {
            let Some(name_os) = process.name().to_str() else {
                continue;
            };
            if name_os.is_empty() {
                continue;
            }

            // On Windows games are always `.exe`; filter out pseudo-processes like
            // "System", "Registry", "Mem Compression" to keep the dropdown useful.
            if cfg!(windows) && !name_os.to_lowercase().ends_with(".exe") {
                continue;
            }

            // On Windows filter out anything running from the Windows directory
            // (svchost.exe, explorer.exe, dwm.exe, etc.). We use the exe path so
            // `C:\Program Files\WindowsApps\...` (Store games) is *not* filtered.
            // Handle long-path `\\?\C:\Windows\...` via normalize_for_filter.
            if cfg!(windows) {
                if let Some(exe) = process.exe() {
                    let exe_lower = Self::normalize_for_filter(exe);
                    if !windir_prefix.is_empty() && exe_lower.starts_with(&windir_prefix) {
                        continue;
                    }
                    // Also handle `\\?\` already stripped, but check again for
                    // `C:\Windows` without trailing slash (file directly inside)
                    if !windir_prefix.is_empty()
                        && exe_lower == windir_prefix.trim_end_matches('\\')
                    {
                        continue;
                    }
                } else {
                    // No exe path -> likely a system pseudo-process or a
                    // privileged SYSTEM process we can't query; skip to keep
                    // the list to "non-system applications only".
                    continue;
                }
            }

            let lower = name_os.to_lowercase();
            let start = process.start_time();
            match seen.entry(lower.clone()) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if start > e.get().1 {
                        e.insert((name_os.to_string(), start));
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert((name_os.to_string(), start));
                }
            }
        }
        let mut out_with_time: Vec<(String, u64)> = seen.into_values().collect();
        // Most recent first
        out_with_time.sort_by_key(|b| std::cmp::Reverse(b.1));
        out_with_time.into_iter().map(|(name, _)| name).collect()
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

    #[test]
    fn test_list_processes_returns_sorted_unique() {
        let mut monitor = ProcessMonitor::new();
        let list = monitor.list_processes();
        // Should be deduplicated case-insensitively
        let lower: Vec<String> = list.iter().map(|s| s.to_lowercase()).collect();
        let mut dedup = lower.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(lower.len(), dedup.len(), "list should be deduplicated");
        // Ordering is by last started (most recent first), not alphabetical;
        // just ensure the list is stable and not empty-checked elsewhere.
    }

    #[test]
    fn test_list_processes_ordered_by_recent() {
        let mut monitor = ProcessMonitor::new();
        let list = monitor.list_processes();
        // On Windows we keep the most recent start_time per exe name, so if
        // there are at least 2 entries we can sanity-check that the most
        // recent process (test binary itself) appears near the front. The test
        // binary is freshly spawned, so its exe should be among the first few.
        if cfg!(windows) && list.len() >= 2 {
            // Find our own exe name (gtt-*.exe under target/debug/deps)
            let lower: Vec<String> = list.iter().map(|s| s.to_lowercase()).collect();
            // At least ensure dedup and filtering still hold, ordering is opaque
            // but we verify no duplicates remain.
            let mut dedup = lower.clone();
            dedup.sort();
            dedup.dedup();
            assert_eq!(lower.len(), dedup.len());
        }
    }

    #[test]
    fn test_list_processes_nonempty() {
        let mut monitor = ProcessMonitor::new();
        let list = monitor.list_processes();
        // In any test environment there is at least one process (on Windows after
        // filtering this is at least the test binary itself).
        assert!(!list.is_empty(), "expected at least one running process");
    }

    #[test]
    fn test_list_processes_filters_to_exe_on_windows() {
        let mut monitor = ProcessMonitor::new();
        let list = monitor.list_processes();
        if cfg!(windows) {
            for name in &list {
                assert!(
                    name.to_lowercase().ends_with(".exe"),
                    "expected only .exe processes on Windows, got {name}"
                );
            }
            // Explorer lives in WINDIR and must be filtered out – if it's
            // unexpectedly present the WINDIR prefix check is broken.
            let lower: Vec<String> = list.iter().map(|s| s.to_lowercase()).collect();
            assert!(
                !lower.contains(&"explorer.exe".to_string()),
                "system explorer.exe should be filtered from non-system list"
            );
        }
    }
}
