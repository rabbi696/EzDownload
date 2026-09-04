//! 操作系统级进程控制模块（挂起/恢复/终止）

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ========== Windows 进程控制 ==========

#[cfg(target_os = "windows")]
mod win32 {
    #[repr(C)]
    pub struct THREADENTRY32 {
        pub dw_size: u32,
        pub cnt_usage: u32,
        pub th32_thread_id: u32,
        pub th32_owner_process_id: u32,
        pub tp_base_pri: i32,
        pub tp_delta_pri: i32,
        pub dw_flags: u32,
    }

    #[repr(C)]
    pub struct PROCESSENTRY32W {
        pub dw_size: u32,
        pub cnt_usage: u32,
        pub th32_process_id: u32,
        pub th32_default_heap_id: usize,
        pub th32_module_id: u32,
        pub cnt_threads: u32,
        pub th32_parent_process_id: u32,
        pub pc_pri_class_base: i32,
        pub dw_flags: u32,
        pub sz_exe_file: [u16; 260],
    }

    pub const TH32CS_SNAPTHREAD: u32 = 0x00000004;
    pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    pub const THREAD_SUSPEND_RESUME: u32 = 0x0002;

    extern "system" {
        pub fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> isize;
        pub fn Thread32First(h_snapshot: isize, lpte: *mut THREADENTRY32) -> i32;
        pub fn Thread32Next(h_snapshot: isize, lpte: *mut THREADENTRY32) -> i32;
        pub fn Process32FirstW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn Process32NextW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn OpenThread(
            dw_desired_access: u32,
            b_inherit_handle: i32,
            dw_thread_id: u32,
        ) -> isize;
        pub fn SuspendThread(h_thread: isize) -> u32;
        pub fn ResumeThread(h_thread: isize) -> u32;
        pub fn CloseHandle(h_object: isize) -> i32;
    }
}

/// 递归收集指定 PID 及其所有子进程的 PID
#[cfg(target_os = "windows")]
fn collect_process_tree(root_pid: u32) -> std::collections::HashSet<u32> {
    let mut pid_set = std::collections::HashSet::new();
    pid_set.insert(root_pid);

    // SAFETY: 调用 Win32 API CreateToolhelp32Snapshot + Process32FirstW/NextW 遍历系统进程表。
    // - snapshot 句柄已检查有效性（!= -1），使用后通过 CloseHandle 释放。
    // - PROCESSENTRY32W 以 zeroed 初始化并正确设置 dw_size，满足 API 前置条件。
    // - 所有指针均指向栈上有效内存，生命周期覆盖整个 unsafe 块。
    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == -1 {
            return pid_set;
        }
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        // 收集所有进程的 (pid, parent_pid)
        let mut all_procs = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                all_procs.push((entry.th32_process_id, entry.th32_parent_process_id));
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);

        // BFS 收集整棵进程树
        let mut queue = vec![root_pid];
        let mut i = 0;
        while i < queue.len() {
            let parent = queue[i];
            for &(pid, ppid) in &all_procs {
                if ppid == parent && pid_set.insert(pid) {
                    queue.push(pid);
                }
            }
            i += 1;
        }
    }
    pid_set
}

/// 挂起指定 PID 的进程及其所有子进程（暂停所有线程）
#[cfg(target_os = "windows")]
pub fn suspend_process(pid: u32) -> Result<(), String> {
    let pids = collect_process_tree(pid);

    // SAFETY: 调用 Win32 API CreateToolhelp32Snapshot + Thread32First/Next 遍历系统线程表。
    // - snapshot 句柄已检查有效性（!= -1），使用后通过 CloseHandle 释放。
    // - THREADENTRY32 以 zeroed 初始化并正确设置 dw_size，满足 API 前置条件。
    // - OpenThread 返回的线程句柄已检查有效性（!= 0），使用后通过 CloseHandle 释放。
    // - SuspendThread 仅挂起目标进程树中的线程，不影响其他进程。
    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == -1 {
            return Err("err_create_thread_snapshot".into());
        }
        let mut entry = std::mem::zeroed::<THREADENTRY32>();
        entry.dw_size = std::mem::size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if pids.contains(&entry.th32_owner_process_id) {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32_thread_id);
                    if thread != 0 {
                        SuspendThread(thread);
                        CloseHandle(thread);
                    }
                }
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Ok(())
    }
}

#[cfg(unix)]
pub fn validate_child_pid(pid: u32) -> Result<(), String> {
    if pid == 0 || pid > i32::MAX as u32 || (pid as i32) <= 0 {
        return Err("err_invalid_pid".to_string());
    }

    let parent_pid = std::process::id();
    let parent_pgid = unsafe { libc::getpgrp() };

    // Never risk operating on or killing the parent application or its process group
    if pid == parent_pid || (pid as i32) == parent_pgid {
        return Err("err_target_is_parent_process".to_string());
    }

    Ok(())
}

#[cfg(unix)]
pub fn verify_child_process_group(pid: u32) -> Result<libc::pid_t, String> {
    validate_child_pid(pid)?;

    let target_pid = pid as libc::pid_t;
    let target_pgid = unsafe { libc::getpgid(target_pid) };
    if target_pgid < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            // Group leader exited; check if the process group still exists with active children
            let group_check = unsafe { libc::kill(-target_pid, 0) };
            if group_check == 0 {
                return Ok(target_pid);
            }
            return Err("err_process_not_found".to_string());
        }
        return Err(format!("err_getpgid_failed:{}", err));
    }

    let parent_pgid = unsafe { libc::getpgrp() };
    if target_pgid == parent_pgid {
        return Err("err_target_in_parent_process_group".to_string());
    }

    // Since our child was spawned with process_group(0), its PGID must equal its PID
    if target_pgid != target_pid {
        return Err("err_stale_pid_mismatch".to_string());
    }

    Ok(target_pgid)
}

#[cfg(unix)]
pub fn suspend_process(pid: u32) -> Result<(), String> {
    let pgid = verify_child_process_group(pid)?;
    if pgid != pid as libc::pid_t {
        return Err("err_stale_pid_mismatch".to_string());
    }

    unsafe {
        libc::kill(-pgid, libc::SIGSTOP);
    }
    Ok(())
}

/// 恢复指定 PID 的进程及其所有子进程（恢复所有线程）
#[cfg(target_os = "windows")]
pub fn resume_process(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("err_invalid_pid".to_string());
    }
    let pids = collect_process_tree(pid);

    // SAFETY: 同 suspend_process，遍历线程表并恢复目标进程树中的所有线程。
    // 所有句柄均经过有效性检查并在使用后关闭。
    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == -1 {
            return Err("err_create_thread_snapshot".into());
        }
        let mut entry = std::mem::zeroed::<THREADENTRY32>();
        entry.dw_size = std::mem::size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if pids.contains(&entry.th32_owner_process_id) {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32_thread_id);
                    if thread != 0 {
                        ResumeThread(thread);
                        CloseHandle(thread);
                    }
                }
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Ok(())
    }
}

#[cfg(unix)]
pub fn resume_process(pid: u32) -> Result<(), String> {
    let pgid = verify_child_process_group(pid)?;
    if pgid != pid as libc::pid_t {
        return Err("err_stale_pid_mismatch".to_string());
    }

    unsafe {
        libc::kill(-pgid, libc::SIGCONT);
    }
    Ok(())
}

/// 终止指定 PID 的进程及其子进程
#[cfg(target_os = "windows")]
pub async fn terminate_process_gracefully(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("err_invalid_pid".to_string());
    }
    use std::os::windows::process::CommandExt;

    // Step 1: Send graceful termination
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    // Step 2: Wait up to 3.5s for process to exit
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(3500);
    let mut exited = false;

    while start.elapsed() < timeout {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let pids = collect_process_tree(pid);
        if pids.is_empty() {
            exited = true;
            break;
        }
    }

    // Step 3: Forceful kill fallback
    if !exited {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    Ok(())
}

#[cfg(unix)]
pub async fn terminate_process_gracefully(pid: u32) -> Result<(), String> {
    let pgid = match verify_child_process_group(pid) {
        Ok(pg) => pg,
        Err(e) => {
            if e == "err_process_not_found" {
                // Entire process group already exited naturally, nothing to kill
                return Ok(());
            }
            return Err(e);
        }
    };

    if pgid != pid as libc::pid_t {
        return Err("err_stale_pid_mismatch".to_string());
    }

    // Step 1: Send SIGTERM to the process group using negative PGID
    unsafe {
        libc::kill(-pgid, libc::SIGTERM);
    }

    // Step 2: Poll group liveness using kill(-pgid, 0) for up to 3.5 seconds
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(3500);
    let mut exited = false;

    while start.elapsed() < timeout {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let res = unsafe { libc::kill(-pgid, 0) };
        if res != 0 {
            let err = std::io::Error::last_os_error().raw_os_error();
            if err == Some(libc::ESRCH) {
                // Entire process group has exited
                exited = true;
                break;
            }
        }
    }

    // Step 3: Fallback to SIGKILL on negative PGID ONLY if process group remains alive
    if !exited {
        // Re-verify that the group is still valid and not parent
        if let Ok(current_pgid) = verify_child_process_group(pid) {
            if current_pgid == pid as libc::pid_t {
                unsafe {
                    libc::kill(-current_pgid, libc::SIGKILL);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn rejects_invalid_pids() {
        assert_eq!(validate_child_pid(0), Err("err_invalid_pid".to_string()));
        assert_eq!(
            validate_child_pid(u32::MAX),
            Err("err_invalid_pid".to_string())
        );
        assert_eq!(
            validate_child_pid(i32::MAX as u32 + 1),
            Err("err_invalid_pid".to_string())
        );
    }

    #[test]
    #[cfg(unix)]
    fn rejects_parent_pid_and_parent_pgid() {
        let parent_pid = std::process::id();
        let parent_pgid = unsafe { libc::getpgrp() };

        assert_eq!(
            validate_child_pid(parent_pid),
            Err("err_target_is_parent_process".to_string())
        );
        if parent_pgid > 0 {
            assert_eq!(
                validate_child_pid(parent_pgid as u32),
                Err("err_target_is_parent_process".to_string())
            );
        }
    }

    #[test]
    #[cfg(unix)]
    fn rejects_stale_pid_or_nonexistent_process() {
        // Find a high PID that does not exist
        let non_existent_pid = 999_999u32;
        let res = verify_child_process_group(non_existent_pid);
        assert_eq!(res, Err("err_process_not_found".to_string()));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn isolated_child_process_group_verified_and_terminated_cleanly() {
        use std::os::unix::process::CommandExt;
        let mut child = std::process::Command::new("sleep")
            .arg("10")
            .process_group(0)
            .spawn()
            .expect("failed to spawn isolated sleep process");

        let child_pid = child.id();
        assert!(child_pid > 0);

        // Verify PGID equals child PID
        let pgid = verify_child_process_group(child_pid).expect("child process group must match");
        assert_eq!(pgid, child_pid as i32);

        // Terminate gracefully (sends SIGTERM first, sleep will exit on SIGTERM within milliseconds)
        let term_res = terminate_process_gracefully(child_pid).await;
        assert!(term_res.is_ok());

        // Wait for OS cleanup
        let _ = child.wait();

        // Calling verify again now returns err_process_not_found
        let after_res = verify_child_process_group(child_pid);
        assert_eq!(after_res, Err("err_process_not_found".to_string()));
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn group_liveness_detects_and_terminates_surviving_child_after_leader_exits() {
        use std::os::unix::process::CommandExt;
        let mut leader = std::process::Command::new("sh")
            .args(["-c", "sleep 30 &"])
            .process_group(0)
            .spawn()
            .expect("failed to spawn leader shell");

        let leader_pid = leader.id();
        assert!(leader_pid > 0);

        // Wait for leader process to exit
        let status = leader.wait().expect("failed to wait on leader");
        assert!(status.success());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Group liveness reports alive via kill(-pgid, 0) even though leader process exited
        let group_alive = unsafe { libc::kill(-(leader_pid as i32), 0) };
        assert_eq!(group_alive, 0);

        let pgid = verify_child_process_group(leader_pid).expect("group must be detected alive");
        assert_eq!(pgid, leader_pid as i32);

        // Terminate gracefully terminates the surviving background child
        let term_res = terminate_process_gracefully(leader_pid).await;
        assert!(term_res.is_ok());

        // Entire process group should now be dead
        let group_dead = unsafe { libc::kill(-(leader_pid as i32), 0) };
        assert_eq!(group_dead, -1);
        let err = std::io::Error::last_os_error().raw_os_error();
        assert_eq!(err, Some(libc::ESRCH));

        let after_res = verify_child_process_group(leader_pid);
        assert_eq!(after_res, Err("err_process_not_found".to_string()));
    }

    #[test]
    #[cfg(unix)]
    fn rejects_child_in_parent_process_group() {
        // Spawning without process_group(0) places child in parent process group
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .expect("failed to spawn non-isolated sleep process");

        let child_pid = child.id();
        let res = verify_child_process_group(child_pid);
        // Clean up child process directly
        let _ = child.kill();
        let _ = child.wait();

        assert_eq!(res, Err("err_target_in_parent_process_group".to_string()));
    }
}
