//! Progress display for downloads using indicatif.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Progress display manager for multiple concurrent downloads.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProgressDisplay {
    multi_progress: MultiProgress,
}

#[allow(dead_code)]
impl ProgressDisplay {
    /// Create a new progress display manager.
    pub fn new() -> Self {
        Self {
            multi_progress: MultiProgress::new(),
        }
    }

    /// Create a new download progress bar for a given total size.
    pub fn download_progress(&self, total: u64) -> DownloadProgress {
        let pb = if total > 0 {
            self.multi_progress.add(ProgressBar::new(total))
        } else {
            self.multi_progress.add(ProgressBar::new_spinner())
        };

        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/cyan}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        DownloadProgress { pb }
    }

    /// Create a new download progress bar with indeterminate progress (spinner).
    pub fn download_progress_indeterminate(&self, msg: &str) -> DownloadProgress {
        let pb = self.multi_progress.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        DownloadProgress { pb }
    }

    /// Clear all progress bars.
    pub fn clear(&self) {
        self.multi_progress.clear().ok();
    }
}

impl Default for ProgressDisplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual download progress bar.
#[derive(Debug)]
#[allow(dead_code)]
pub struct DownloadProgress {
    pb: ProgressBar,
}

#[allow(dead_code)]
impl DownloadProgress {
    /// Update the progress bar with current download state.
    ///
    /// - `downloaded`: Number of bytes downloaded so far
    /// - `total`: Total bytes to download (0 if unknown)
    /// - `speed`: Current download speed in bytes per second
    /// - `eta`: Estimated time remaining in seconds
    pub fn update(&self, downloaded: u64, total: u64, speed: f64, eta: f64) {
        if total > 0 {
            self.pb.set_length(total);
            self.pb.set_position(downloaded);

            let speed_str = format_speed(speed);
            let eta_str = format_eta(eta);

            self.pb.set_prefix(format!(
                "{:.1}%",
                (downloaded as f64 / total as f64) * 100.0
            ));
            self.pb
                .set_message(format!("{} | ETA: {}", speed_str, eta_str));
        }
    }

    /// Update progress with just downloaded bytes (for indeterminate size).
    pub fn update_bytes(&self, downloaded: u64, speed: f64) {
        self.pb.set_position(downloaded);
        let speed_str = format_speed(speed);
        self.pb.set_message(format!("{} downloaded", speed_str));
    }

    /// Finish the progress bar.
    pub fn finish(&self) {
        self.pb.finish_with_message("Done");
    }

    /// Finish with completion message.
    pub fn finish_with_message(&self, msg: &str) {
        self.pb.finish_with_message(msg.to_string());
    }

    /// Set the current status message.
    pub fn set_message(&self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }

    /// Get the underlying progress bar (for advanced use).
    pub fn inner(&self) -> &ProgressBar {
        &self.pb
    }
}

/// Format bytes per second into human-readable string.
fn format_speed(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.1} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.1} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// Format ETA in seconds to human-readable string.
fn format_eta(eta_secs: f64) -> String {
    if eta_secs.is_infinite() || eta_secs.is_nan() {
        return "---".to_string();
    }

    let duration = Duration::from_secs(eta_secs.ceil() as u64);

    if duration.as_secs() >= 3600 {
        format!(
            "{}h {}m",
            duration.as_secs() / 3600,
            (duration.as_secs() % 3600) / 60
        )
    } else if duration.as_secs() >= 60 {
        format!("{}m {}s", duration.as_secs() / 60, duration.as_secs() % 60)
    } else {
        format!("{}s", duration.as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(500.0), "500 B/s");
        assert_eq!(format_speed(1024.0), "1.0 KB/s");
        assert_eq!(format_speed(1048576.0), "1.0 MB/s");
        assert_eq!(format_speed(1073741824.0), "1.0 GB/s");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(30.0), "30s");
        assert_eq!(format_eta(90.0), "1m 30s");
        assert_eq!(format_eta(3660.0), "1h 1m");
        assert_eq!(format_eta(f64::INFINITY), "---");
        assert_eq!(format_eta(f64::NAN), "---");
    }
}
