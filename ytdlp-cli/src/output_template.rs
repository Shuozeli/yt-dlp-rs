//! Output template system for formatting video information and download paths.

use std::collections::HashMap;
use ytdlp_proto::proto::{Format, VideoInfo};

/// Output template for formatting video information and download paths.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OutputTemplate {
    template: String,
}

#[allow(dead_code)]
impl OutputTemplate {
    /// Create a new output template from a template string.
    ///
    /// Supports the following placeholders:
    /// - `%(id)s` - Video ID
    /// - `%(title)s` - Video title
    /// - `%(ext)s` - File extension
    /// - `%(uploader)s` - Uploader name
    /// - `%(resolution)s` - Video resolution
    /// - `%(duration)s` - Duration in seconds
    /// - `%(upload_date)s` - Upload date (from metadata)
    /// - `%(width)s` - Video width
    /// - `%(height)s` - Video height
    pub fn new(template: &str) -> Self {
        Self {
            template: template.to_string(),
        }
    }

    /// Apply the template to a video info, returning the formatted string.
    pub fn apply(&self, video_info: &VideoInfo, format: Option<&Format>) -> String {
        let mut replacements = HashMap::new();

        replacements.insert("id".to_string(), video_info.id.clone());
        replacements.insert("title".to_string(), video_info.title.clone());
        replacements.insert("uploader".to_string(), video_info.uploader.clone());
        replacements.insert("duration".to_string(), video_info.duration.to_string());

        // Get upload date from metadata if available
        if let Some(date) = video_info.metadata.get("upload_date") {
            replacements.insert("upload_date".to_string(), date.clone());
        } else {
            replacements.insert("upload_date".to_string(), "unknown".to_string());
        }

        // Handle format-specific fields
        if let Some(f) = format {
            replacements.insert("ext".to_string(), f.ext.clone());
            replacements.insert("resolution".to_string(), f.resolution.clone());
            replacements.insert("format_id".to_string(), f.format_id.clone());

            // Try to extract width/height from resolution (e.g., "1920x1080")
            let res = &f.resolution;
            let parts: Vec<&str> = res.split('x').collect();
            if parts.len() == 2 {
                replacements.insert("width".to_string(), parts[0].to_string());
                replacements.insert("height".to_string(), parts[1].to_string());
            } else {
                replacements.insert("width".to_string(), "0".to_string());
                replacements.insert("height".to_string(), "0".to_string());
            }
        } else {
            replacements.insert("ext".to_string(), "mp4".to_string());
            replacements.insert("resolution".to_string(), "unknown".to_string());
            replacements.insert("format_id".to_string(), "best".to_string());
            replacements.insert("width".to_string(), "0".to_string());
            replacements.insert("height".to_string(), "0".to_string());
        }

        let mut result = self.template.clone();

        for (key, value) in replacements {
            let placeholder = format!("%({})s", key);
            result = result.replace(&placeholder, &value);
        }

        // Sanitize filename by removing invalid characters
        for c in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
            result = result.replace(c, "_");
        }

        result
    }

    /// Get the template string.
    pub fn template(&self) -> &str {
        &self.template
    }
}

impl Default for OutputTemplate {
    fn default() -> Self {
        Self::new("%(title)s-%(id)s.%(ext)s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_video_info() -> VideoInfo {
        VideoInfo {
            id: "abc123".to_string(),
            title: "Test Video Title".to_string(),
            description: "A test video".to_string(),
            uploader: "TestUploader".to_string(),
            uploader_url: "https://example.com/user".to_string(),
            duration: 3600,
            metadata: HashMap::new(),
            formats: vec![],
            subtitles: vec![],
            thumbnails: vec![],
        }
    }

    #[test]
    fn test_basic_template() {
        let template = OutputTemplate::new("%(title)s-%(id)s.%(ext)s");
        let video = create_test_video_info();
        let result = template.apply(&video, None);
        assert_eq!(result, "Test Video Title-abc123.mp4");
    }

    #[test]
    fn test_template_sanitization() {
        let template = OutputTemplate::new("%(title)s/%(id)s.%(ext)s");
        let video = create_test_video_info();
        let result = template.apply(&video, None);
        assert_eq!(result, "Test Video Title_abc123.mp4");
    }

    #[test]
    fn test_default_template() {
        let template = OutputTemplate::default();
        let video = create_test_video_info();
        let result = template.apply(&video, None);
        assert_eq!(result, "Test Video Title-abc123.mp4");
    }
}
