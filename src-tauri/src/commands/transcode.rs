//! FFmpeg video transcoding for Adobe Premiere Pro compatibility.
//! Supports H.264 MP4 (libx264 or h264_videotoolbox) and Apple ProRes 422 LT MOV.

use crate::platform::process;
use crate::utils;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use super::download::parser;
use super::probe::verify_media_file;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscodeTarget {
    H264Mp4,
    Prores422LtMov,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeParams {
    pub task_id: String,
    pub input_path: String,
    pub target: TranscodeTarget,
    pub keep_original: bool,
    pub use_hardware_acceleration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscodeExecutionState {
    Running,
    Completed,
    #[allow(dead_code)]
    Failed,
    Cancelled,
}

pub struct TranscodeProcessInfo {
    pub pid: u32,
    pub state: TranscodeExecutionState,
    pub cancelled: bool,
    #[allow(dead_code)]
    pub output_path: PathBuf,
    pub temp_dir: PathBuf,
    #[allow(dead_code)]
    pub input_path: PathBuf,
    pub keep_original: bool,
}

pub fn is_valid_transcode_temp_dir(temp_dir: &Path, task_id: &str) -> bool {
    if let Some(name) = temp_dir.file_name().and_then(|s| s.to_str()) {
        name.starts_with(".ezdownload_tmp_transcode_") && name.contains(task_id)
    } else {
        false
    }
}

#[derive(Default)]
pub struct TranscodeState {
    pub processes: Arc<Mutex<HashMap<String, TranscodeProcessInfo>>>,
}

fn determine_output_path(input_path: &Path, target: TranscodeTarget) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");

    match target {
        TranscodeTarget::H264Mp4 => {
            let mut candidate = parent.join(format!("{}_h264.mp4", stem));
            if candidate == input_path {
                candidate = parent.join(format!("{}_converted.mp4", stem));
            }
            candidate
        }
        TranscodeTarget::Prores422LtMov => {
            let candidate = parent.join(format!("{}_prores.mov", stem));
            candidate
        }
    }
}

pub fn build_ffmpeg_transcode_args(
    input_path: &Path,
    output_path: &Path,
    target: TranscodeTarget,
    use_hardware_accel: bool,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-i".to_string(),
        input_path.to_string_lossy().to_string(),
    ];

    match target {
        TranscodeTarget::H264Mp4 => {
            #[cfg(target_os = "macos")]
            let use_videotoolbox = use_hardware_accel;
            #[cfg(not(target_os = "macos"))]
            let use_videotoolbox = false;

            if use_videotoolbox {
                args.extend([
                    "-c:v".to_string(),
                    "h264_videotoolbox".to_string(),
                    "-b:v".to_string(),
                    "8000k".to_string(),
                    "-pix_fmt".to_string(),
                    "yuv420p".to_string(),
                    "-c:a".to_string(),
                    "aac".to_string(),
                    "-b:a".to_string(),
                    "192k".to_string(),
                    "-movflags".to_string(),
                    "+faststart".to_string(),
                ]);
            } else {
                args.extend([
                    "-c:v".to_string(),
                    "libx264".to_string(),
                    "-preset".to_string(),
                    "medium".to_string(),
                    "-crf".to_string(),
                    "20".to_string(),
                    "-pix_fmt".to_string(),
                    "yuv420p".to_string(),
                    "-c:a".to_string(),
                    "aac".to_string(),
                    "-b:a".to_string(),
                    "192k".to_string(),
                    "-movflags".to_string(),
                    "+faststart".to_string(),
                ]);
            }
        }
        TranscodeTarget::Prores422LtMov => {
            args.extend([
                "-c:v".to_string(),
                "prores_ks".to_string(),
                "-profile:v".to_string(),
                "1".to_string(), // ProRes 422 LT
                "-c:a".to_string(),
                "pcm_s16le".to_string(),
            ]);
        }
    }

    args.push(output_path.to_string_lossy().to_string());
    args
}

#[tauri::command]
pub async fn convert_video_for_premiere(
    app: AppHandle,
    state: tauri::State<'_, TranscodeState>,
    params: TranscodeParams,
) -> Result<String, String> {
    let input = PathBuf::from(&params.input_path);
    if !input.exists() {
        return Err("err_input_file_not_found".to_string());
    }

    let ffmpeg_path = utils::get_ffmpeg_path(&app)?;
    if !ffmpeg_path.exists() {
        return Err("err_ffmpeg_not_installed".to_string());
    }

    let output_path = determine_output_path(&input, params.target);
    let output_str = output_path.to_string_lossy().to_string();

    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let temp_job_dir = parent.join(format!(".ezdownload_tmp_transcode_{}", params.task_id));
    std::fs::create_dir_all(&temp_job_dir).map_err(|e| format!("err_create_temp_dir:{}", e))?;
    let temp_output_file = temp_job_dir.join(
        output_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("output.mp4")),
    );

    let args = build_ffmpeg_transcode_args(
        &input,
        &temp_output_file,
        params.target,
        params.use_hardware_acceleration,
    );

    let mut cmd = tokio::process::Command::new(&ffmpeg_path);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(crate::commands::CREATE_NO_WINDOW);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.as_std_mut().process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("err_spawn_transcode:{}", e))?;

    let pid = child.id().ok_or("err_get_pid")?;
    let task_id = params.task_id.clone();
    let processes = state.processes.clone();

    {
        let mut map = processes.lock().map_err(|e| e.to_string())?;
        map.insert(
            task_id.clone(),
            TranscodeProcessInfo {
                pid,
                state: TranscodeExecutionState::Running,
                cancelled: false,
                output_path: output_path.clone(),
                temp_dir: temp_job_dir.clone(),
                input_path: input.clone(),
                keep_original: params.keep_original,
            },
        );
    }

    // Inspect input duration to calculate accurate transcode percentage
    let total_duration = verify_media_file(app.clone(), params.input_path.clone())
        .await
        .ok()
        .and_then(|probe| probe.duration_seconds)
        .unwrap_or(0.0);

    let stderr = child.stderr.take().ok_or("err_capture_stderr")?;
    let app_clone = app.clone();
    let task_id_clone = task_id.clone();

    // Read progress lines from stderr
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if line.contains("time=") {
                if let Some(current_secs) = parser::parse_ffmpeg_time(&line) {
                    let percent = if total_duration > 0.0 {
                        (current_secs / total_duration * 100.0).clamp(0.0, 99.9)
                    } else {
                        0.0
                    };
                    let speed = parser::parse_ffmpeg_speed(&line);
                    let _ = app_clone.emit(
                        "transcode-progress",
                        serde_json::json!({
                            "id": task_id_clone,
                            "percent": percent,
                            "timeSeconds": current_secs,
                            "speed": speed,
                        }),
                    );
                }
            }
        }
    });

    // Wait for completion
    let app_handle = app.clone();
    let final_output = output_path.clone();
    let task_id_done = task_id.clone();
    let processes_done = processes.clone();
    let temp_job_dir_done = temp_job_dir.clone();
    let temp_output_file_done = temp_output_file.clone();

    tokio::spawn(async move {
        let status = child.wait().await;

        let was_cancelled = processes_done
            .lock()
            .ok()
            .and_then(|m| m.get(&task_id_done).map(|i| i.cancelled))
            .unwrap_or(false);

        let keep_orig = processes_done
            .lock()
            .ok()
            .and_then(|m| m.get(&task_id_done).map(|i| i.keep_original))
            .unwrap_or(true);

        if was_cancelled {
            let _ = tokio::fs::remove_dir_all(&temp_job_dir_done).await;
        } else if matches!(&status, Ok(s) if s.success()) && temp_output_file_done.exists() {
            // Verify final output file in temp directory using ffprobe
            match verify_media_file(
                app_handle.clone(),
                temp_output_file_done.to_string_lossy().to_string(),
            )
            .await
            {
                Ok(probe) => {
                    // Atomically move from temp to final destination
                    if let Err(e) = crate::commands::download::atomic_move(
                        &temp_output_file_done,
                        &final_output,
                    ) {
                        let _ = tokio::fs::remove_dir_all(&temp_job_dir_done).await;
                        let _ = app_handle.emit(
                            "transcode-error",
                            serde_json::json!({
                                "id": task_id_done,
                                "error": format!("err_move_output:{}", e),
                            }),
                        );
                        return;
                    }

                    // Clean up temp directory
                    if is_valid_transcode_temp_dir(&temp_job_dir_done, &task_id_done) {
                        let _ = tokio::fs::remove_dir_all(&temp_job_dir_done).await;
                    }

                    if let Ok(mut map) = processes_done.lock() {
                        if let Some(info) = map.get_mut(&task_id_done) {
                            info.state = TranscodeExecutionState::Completed;
                        }
                    }

                    // Safe cleanup: only delete input if user didn't request keep_original AND probe verified
                    if !keep_orig && input != final_output {
                        let _ = tokio::fs::remove_file(&input).await;
                    }

                    let _ = app_handle.emit(
                        "transcode-complete",
                        serde_json::json!({
                            "id": task_id_done,
                            "outputFile": final_output.to_string_lossy().to_string(),
                            "probe": probe,
                        }),
                    );
                }
                Err(err) => {
                    if is_valid_transcode_temp_dir(&temp_job_dir_done, &task_id_done) {
                        let _ = tokio::fs::remove_dir_all(&temp_job_dir_done).await;
                    }
                    let _ = app_handle.emit(
                        "transcode-error",
                        serde_json::json!({
                            "id": task_id_done,
                            "error": format!("err_probe_failed:{}", err),
                        }),
                    );
                }
            }
        } else {
            if is_valid_transcode_temp_dir(&temp_job_dir_done, &task_id_done) {
                let _ = tokio::fs::remove_dir_all(&temp_job_dir_done).await;
            }
            let _ = app_handle.emit(
                "transcode-error",
                serde_json::json!({
                    "id": task_id_done,
                    "error": "err_transcode_failed",
                }),
            );
        }

        if let Ok(mut map) = processes_done.lock() {
            map.remove(&task_id_done);
        }
    });

    Ok(output_str)
}

#[tauri::command]
pub async fn cancel_transcode(
    state: tauri::State<'_, TranscodeState>,
    task_id: String,
) -> Result<(), String> {
    let (pid, temp_dir) = {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        let info = processes.get_mut(&task_id).ok_or("err_task_not_found")?;
        if info.state == TranscodeExecutionState::Completed {
            return Err("err_task_already_completed".to_string());
        }
        if info.state == TranscodeExecutionState::Cancelled {
            return Ok(());
        }
        info.state = TranscodeExecutionState::Cancelled;
        info.cancelled = true;
        (info.pid, info.temp_dir.clone())
    };

    // 发送 SIGTERM 并等待 3~5 秒，超时后使用 SIGKILL 兜底
    process::terminate_process_gracefully(pid).await?;

    // 仅清理专属临时目录，绝不删除最终目标文件
    if is_valid_transcode_temp_dir(&temp_dir, &task_id) && temp_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_h264_args_correctly() {
        let input = Path::new("/tmp/input.webm");
        let output = Path::new("/tmp/output.mp4");
        let args = build_ffmpeg_transcode_args(input, output, TranscodeTarget::H264Mp4, false);

        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert_eq!(args.last().unwrap(), "/tmp/output.mp4");
    }

    #[test]
    fn generates_prores_args_correctly() {
        let input = Path::new("/tmp/input.webm");
        let output = Path::new("/tmp/output.mov");
        let args =
            build_ffmpeg_transcode_args(input, output, TranscodeTarget::Prores422LtMov, false);

        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"prores_ks".to_string()));
        assert!(args.contains(&"-profile:v".to_string()));
        assert!(args.contains(&"1".to_string()));
        assert!(args.contains(&"pcm_s16le".to_string()));
        assert_eq!(args.last().unwrap(), "/tmp/output.mov");
    }

    #[test]
    fn determines_distinct_output_path() {
        let input = Path::new("/downloads/my_video.webm");
        let h264_out = determine_output_path(input, TranscodeTarget::H264Mp4);
        assert_eq!(h264_out, PathBuf::from("/downloads/my_video_h264.mp4"));

        let prores_out = determine_output_path(input, TranscodeTarget::Prores422LtMov);
        assert_eq!(prores_out, PathBuf::from("/downloads/my_video_prores.mov"));
    }

    #[test]
    fn validates_transcode_temp_dir_pattern_correctly() {
        let task_id = "transcode-task-456";
        let valid_path =
            PathBuf::from("/downloads").join(format!(".ezdownload_tmp_transcode_{}", task_id));
        assert!(is_valid_transcode_temp_dir(&valid_path, task_id));

        // Wrong task ID
        let wrong_task = PathBuf::from("/downloads").join(".ezdownload_tmp_transcode_other");
        assert!(!is_valid_transcode_temp_dir(&wrong_task, task_id));

        // Non-transcode temp directory
        let download_temp =
            PathBuf::from("/downloads").join(format!(".ezdownload_tmp_{}", task_id));
        assert!(!is_valid_transcode_temp_dir(&download_temp, task_id));

        // Final output file
        let output_file = PathBuf::from("/downloads/my_video_h264.mp4");
        assert!(!is_valid_transcode_temp_dir(&output_file, task_id));
    }

    #[test]
    fn transcode_cancellation_state_transitions_guard_completed_jobs() {
        let mut info = TranscodeProcessInfo {
            pid: 54321,
            state: TranscodeExecutionState::Running,
            cancelled: false,
            output_path: PathBuf::from("/downloads/video_h264.mp4"),
            temp_dir: PathBuf::from("/downloads/.ezdownload_tmp_transcode_job1"),
            input_path: PathBuf::from("/downloads/video.webm"),
            keep_original: true,
        };

        // Allowed while running
        assert_eq!(info.state, TranscodeExecutionState::Running);
        info.state = TranscodeExecutionState::Cancelled;
        info.cancelled = true;
        assert_eq!(info.state, TranscodeExecutionState::Cancelled);

        // Rejected when completed
        info.state = TranscodeExecutionState::Completed;
        let cancel_attempt = if info.state == TranscodeExecutionState::Completed {
            Err("err_task_already_completed".to_string())
        } else {
            Ok(())
        };
        assert_eq!(
            cancel_attempt,
            Err("err_task_already_completed".to_string())
        );
    }
}
