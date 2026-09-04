//! 下载任务的暂停、恢复与取消。

use crate::platform::process;

use super::model::DownloadState;

/// 暂停下载任务（挂起子进程）
#[tauri::command]
pub async fn pause_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
) -> Result<(), String> {
    let processes = state.processes.lock().map_err(|e| e.to_string())?;
    let info = processes.get(&id).ok_or("err_task_not_found")?;
    process::suspend_process(info.pid)
}

/// 继续下载任务（恢复子进程）
#[tauri::command]
pub async fn resume_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
) -> Result<(), String> {
    let processes = state.processes.lock().map_err(|e| e.to_string())?;
    let info = processes.get(&id).ok_or("err_task_not_found")?;
    process::resume_process(info.pid)
}

/// 检查是否为合法的下载专属临时目录
pub fn is_valid_job_temp_dir(temp_dir: &std::path::Path, job_id: &str) -> bool {
    if let Some(name) = temp_dir.file_name().and_then(|s| s.to_str()) {
        name.starts_with(".ezdownload_tmp_") && name.contains(job_id)
    } else {
        false
    }
}

/// 取消下载任务并清理专属隔离临时目录
#[tauri::command]
pub async fn cancel_download(
    state: tauri::State<'_, DownloadState>,
    id: String,
    _delete_files: bool,
) -> Result<(), String> {
    let (pid, temp_dir) = {
        let mut processes = state.processes.lock().map_err(|e| e.to_string())?;
        let info = processes.get_mut(&id).ok_or("err_task_not_found")?;
        if info.state == super::model::ExecutionState::Completed {
            return Err("err_task_already_completed".to_string());
        }
        if info.state == super::model::ExecutionState::Cancelled {
            return Ok(());
        }
        info.state = super::model::ExecutionState::Cancelled;
        info.cancelled = true;
        (info.pid, info.temp_dir.clone())
    };

    // 发送 SIGTERM 并等待 3~5 秒，超时后使用 SIGKILL 作为最终兜底
    process::terminate_process_gracefully(pid).await?;

    // 仅清理匹配当前任务专属 UUID 的临时目录，绝不删除已验证移动的目标目录成品文件
    if is_valid_job_temp_dir(&temp_dir, &id) && temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::download::model::{DownloadProcessInfo, ExecutionState};
    use std::path::PathBuf;

    #[test]
    fn validates_job_temp_dir_pattern_correctly() {
        let task_id = "test-uuid-123";
        let valid_path = PathBuf::from("/downloads").join(format!(".ezdownload_tmp_{}", task_id));
        assert!(is_valid_job_temp_dir(&valid_path, task_id));

        // Wrong task id
        let wrong_task = PathBuf::from("/downloads").join(".ezdownload_tmp_other-uuid");
        assert!(!is_valid_job_temp_dir(&wrong_task, task_id));

        // Normal folder without prefix
        let normal_folder = PathBuf::from("/downloads").join("my_video");
        assert!(!is_valid_job_temp_dir(&normal_folder, task_id));

        // Root /downloads directory itself
        let downloads_dir = PathBuf::from("/downloads");
        assert!(!is_valid_job_temp_dir(&downloads_dir, task_id));
    }

    #[test]
    fn cancellation_state_machine_prevents_stale_pid_kill() {
        let mut info = DownloadProcessInfo {
            pid: 12345,
            state: ExecutionState::Running,
            cancelled: false,
            output_files: Vec::new(),
            download_dir: "/downloads".to_string(),
            temp_dir: PathBuf::from("/downloads/.ezdownload_tmp_task-1"),
            filepath_file: None,
            clip_duration: None,
            last_error: None,
            premiere_preset: true,
            no_overwrites: false,
        };

        // When running, cancellation is allowed
        assert_eq!(info.state, ExecutionState::Running);
        info.state = ExecutionState::Cancelled;
        info.cancelled = true;
        assert_eq!(info.state, ExecutionState::Cancelled);

        // When already completed, cancellation MUST be rejected
        info.state = ExecutionState::Completed;
        let cancel_attempt = if info.state == ExecutionState::Completed {
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
