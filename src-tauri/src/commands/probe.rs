//! Media stream probe and verification using ffprobe JSON output.

use crate::utils;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbeResult {
    pub path: String,
    pub exists: bool,
    pub format_name: String,
    pub container_extension: String,
    pub video_codec: Option<String>,
    pub video_codec_tag: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub file_size_bytes: Option<u64>,
    pub is_premiere_ready: bool,
    pub compatibility_label: String,
    pub incompatibility_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_name: Option<String>,
    codec_type: Option<String>,
    codec_tag_string: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
}

fn parse_frame_rate(rate: &str) -> Option<f64> {
    if rate.is_empty() || rate == "0/0" {
        return None;
    }
    if let Some((num, den)) = rate.split_once('/') {
        let n: f64 = num.trim().parse().ok()?;
        let d: f64 = den.trim().parse().ok()?;
        if d > 0.0 {
            Some(n / d)
        } else {
            None
        }
    } else {
        rate.parse().ok()
    }
}

pub fn evaluate_premiere_compatibility(
    container: &str,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
) -> (bool, String, Option<String>) {
    let lower_container = container.to_lowercase();
    let vcodec = video_codec.unwrap_or("").to_lowercase();
    let acodec = audio_codec.unwrap_or("").to_lowercase();

    // Container checks
    let is_mp4_or_mov = lower_container.contains("mp4")
        || lower_container.contains("mov")
        || lower_container.contains("m4v")
        || lower_container.contains("m4a");

    let is_webm = lower_container.contains("webm") || lower_container.contains("matroska");

    // Video codec checks
    let is_h264 = vcodec == "h264" || vcodec.starts_with("avc1") || vcodec.starts_with("avc3");
    let is_prores = vcodec == "prores" || vcodec.starts_with("apc");
    let is_av1 = vcodec == "av1" || vcodec.starts_with("av01");
    let is_vp9 = vcodec == "vp9" || vcodec.starts_with("vp09") || vcodec == "vp8";

    // Audio codec checks
    let is_compatible_audio = acodec.is_empty()
        || acodec == "aac"
        || acodec.starts_with("mp4a")
        || acodec.starts_with("pcm_")
        || acodec == "mp3";

    if is_webm && (is_vp9 || is_av1) {
        return (
            false,
            "Conversion recommended".to_string(),
            Some(format!(
                "WebM container with {} video codec cannot be imported directly into Premiere Pro.",
                if is_av1 { "AV1" } else { "VP9" }
            )),
        );
    }

    if is_av1 {
        return (
            false,
            "Conversion recommended".to_string(),
            Some("AV1 (av01) video codec is not supported natively in Adobe Premiere Pro. Transcoding to H.264 or ProRes is recommended.".to_string()),
        );
    }

    if is_vp9 {
        return (
            false,
            "Conversion recommended".to_string(),
            Some("VP9 (vp09) video codec is not supported natively in Adobe Premiere Pro. Transcoding to H.264 or ProRes is recommended.".to_string()),
        );
    }

    if is_h264 {
        if !is_mp4_or_mov {
            return (
                false,
                "Conversion recommended".to_string(),
                Some(format!("H.264 video in a {} container may experience import issues in Premiere Pro. MP4 or MOV is recommended.", container)),
            );
        }
        if !is_compatible_audio {
            return (
                false,
                "Conversion recommended".to_string(),
                Some(format!("Audio codec '{}' may not decode reliably in Premiere Pro. AAC or PCM is recommended.", acodec)),
            );
        }
        return (true, "Premiere Ready".to_string(), None);
    }

    if is_prores {
        return (true, "Premiere Ready".to_string(), None);
    }

    if vcodec.is_empty() && !acodec.is_empty() {
        // Audio-only file
        if acodec == "aac"
            || acodec.starts_with("mp4a")
            || acodec.starts_with("pcm_")
            || acodec == "mp3"
        {
            return (true, "Premiere Ready (Audio)".to_string(), None);
        }
        return (
            false,
            "Conversion recommended".to_string(),
            Some(format!(
                "Audio format {} may require conversion for Premiere Pro.",
                acodec
            )),
        );
    }

    (
        false,
        "Unknown compatibility".to_string(),
        Some(format!(
            "Video codec '{}' in container '{}' is not recognized as standard Premiere Pro format.",
            vcodec, container
        )),
    )
}

pub fn parse_probe_json(raw_json: &str, file_path: &str) -> Result<MediaProbeResult, String> {
    let parsed: FfprobeOutput =
        serde_json::from_str(raw_json).map_err(|e| format!("err_parse_ffprobe_json:{}", e))?;

    let path_obj = Path::new(file_path);
    let container_ext = path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let streams = parsed.streams.unwrap_or_default();
    let video_stream = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let audio_stream = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"));

    let video_codec = video_stream.and_then(|s| s.codec_name.clone());
    let video_codec_tag = video_stream.and_then(|s| s.codec_tag_string.clone());
    let audio_codec = audio_stream.and_then(|s| s.codec_name.clone());

    let width = video_stream.and_then(|s| s.width);
    let height = video_stream.and_then(|s| s.height);
    let fps = video_stream
        .and_then(|s| s.r_frame_rate.as_deref())
        .and_then(parse_frame_rate);

    let format = parsed.format.unwrap_or(FfprobeFormat {
        format_name: None,
        duration: None,
        size: None,
    });

    let format_name = format.format_name.unwrap_or_else(|| container_ext.clone());
    let duration_seconds = format
        .duration
        .as_deref()
        .or_else(|| video_stream.and_then(|s| s.duration.as_deref()))
        .and_then(|d| d.parse::<f64>().ok());

    let file_size_bytes = format
        .size
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| std::fs::metadata(file_path).ok().map(|m| m.len()));

    let (is_premiere_ready, compatibility_label, incompatibility_reason) =
        evaluate_premiere_compatibility(
            &format_name,
            video_codec.as_deref(),
            audio_codec.as_deref(),
        );

    Ok(MediaProbeResult {
        path: file_path.to_string(),
        exists: path_obj.exists(),
        format_name,
        container_extension: container_ext,
        video_codec,
        video_codec_tag,
        audio_codec,
        width,
        height,
        fps,
        duration_seconds,
        file_size_bytes,
        is_premiere_ready,
        compatibility_label,
        incompatibility_reason,
    })
}

#[tauri::command]
pub async fn verify_media_file(app: AppHandle, path: String) -> Result<MediaProbeResult, String> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err("err_file_not_found".to_string());
    }

    let ffprobe = utils::get_ffprobe_path(&app)?;
    if !ffprobe.exists() {
        return Err("err_ffprobe_not_installed".to_string());
    }

    let mut cmd = tokio::process::Command::new(ffprobe);
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        &path,
    ]);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(crate::commands::CREATE_NO_WINDOW);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.as_std_mut().process_group(0);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("err_execute_ffprobe:{}", e))?;

    if !output.status.success() {
        return Err(format!(
            "err_ffprobe_failed:{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    parse_probe_json(&raw, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_h264_mp4_as_premiere_ready() {
        let (ready, label, reason) =
            evaluate_premiere_compatibility("mov,mp4,m4a,3gp,3g2,mj2", Some("h264"), Some("aac"));
        assert!(ready);
        assert_eq!(label, "Premiere Ready");
        assert!(reason.is_none());
    }

    #[test]
    fn identifies_prores_mov_as_premiere_ready() {
        let (ready, label, reason) = evaluate_premiere_compatibility(
            "mov,mp4,m4a,3gp,3g2,mj2",
            Some("prores"),
            Some("pcm_s16le"),
        );
        assert!(ready);
        assert_eq!(label, "Premiere Ready");
        assert!(reason.is_none());
    }

    #[test]
    fn rejects_av1_mp4_with_clear_warning() {
        let (ready, label, reason) =
            evaluate_premiere_compatibility("mov,mp4,m4a,3gp,3g2,mj2", Some("av1"), Some("aac"));
        assert!(!ready);
        assert_eq!(label, "Conversion recommended");
        assert!(reason.unwrap().contains("AV1"));
    }

    #[test]
    fn rejects_vp9_webm_with_clear_warning() {
        let (ready, label, reason) =
            evaluate_premiere_compatibility("matroska,webm", Some("vp9"), Some("opus"));
        assert!(!ready);
        assert_eq!(label, "Conversion recommended");
        assert!(reason.unwrap().contains("VP9"));
    }

    #[test]
    fn parses_ffprobe_json_correctly() {
        let json = r#"{
            "streams": [
                {
                    "index": 0,
                    "codec_name": "h264",
                    "codec_type": "video",
                    "codec_tag_string": "avc1",
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "30000/1001",
                    "duration": "120.5"
                },
                {
                    "index": 1,
                    "codec_name": "aac",
                    "codec_type": "audio",
                    "codec_tag_string": "mp4a",
                    "duration": "120.5"
                }
            ],
            "format": {
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
                "duration": "120.5",
                "size": "52428800"
            }
        }"#;

        let result = parse_probe_json(json, "/tmp/video.mp4").unwrap();
        assert!(result.is_premiere_ready);
        assert_eq!(result.video_codec.as_deref(), Some("h264"));
        assert_eq!(result.audio_codec.as_deref(), Some("aac"));
        assert_eq!(result.width, Some(1920));
        assert_eq!(result.height, Some(1080));
        assert!((result.fps.unwrap() - 29.97).abs() < 0.01);
        assert_eq!(result.duration_seconds, Some(120.5));
        assert_eq!(result.file_size_bytes, Some(52428800));
    }
}
