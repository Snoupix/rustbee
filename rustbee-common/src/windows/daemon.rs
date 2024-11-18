use std::ffi::CStr;
use std::io;
use std::mem::size_of;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command as AsyncCommand;
use tokio::time;
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS, PROCESS_TERMINATE,
};

/// Maps a windows::core::Error into std::io::Error
macro_rules! werr {
    ($res:expr) => {
        $res.map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))
    };
}

fn get_daemon_process_id() -> io::Result<Option<u32>> {
    unsafe {
        let snapshot = werr!(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0))?;

        if snapshot == HANDLE::default() {
            return Err(io::Error::last_os_error());
        }

        let mut entry = PROCESSENTRY32 {
            dwSize: size_of::<PROCESSENTRY32>() as _,
            ..Default::default()
        };

        werr!(Process32First(snapshot, &mut entry))?;

        loop {
            let process_name = CStr::from_ptr(entry.szExeFile.as_ptr())
                .to_string_lossy()
                .into_owned();

            if process_name == "rustbee-daemon.exe" {
                return Ok(Some(entry.th32ProcessID));
            }

            if Process32Next(snapshot, &mut entry).is_err() {
                break;
            }
        }
    }

    Ok(None)
}

pub async fn launch_daemon() -> io::Result<()> {
    let pid_opt = get_daemon_process_id()?;

    if pid_opt.is_some() {
        return Ok(());
    }

    let daemon = AsyncCommand::new("rustbee-daemon.exe")
        .creation_flags(DETACHED_PROCESS.0 | CREATE_NEW_PROCESS_GROUP.0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let out = match time::timeout(Duration::from_secs(1), daemon.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => return Ok(()),
    };

    if !out.status.success() {
        let stderr = String::from_utf8(out.stderr).unwrap();
        let stderr = stderr.trim();

        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("[ERROR] Failed to launch rustbee-daemon:\n{stderr}"),
        ));
    }

    Ok(())
}

pub fn shutdown_daemon(_force: bool) -> io::Result<()> {
    let pid_opt = get_daemon_process_id()?;

    if let Some(pid) = pid_opt {
        // if force {
        unsafe {
            let process_handle = werr!(OpenProcess(PROCESS_TERMINATE, BOOL(false as _), pid))?;
            if process_handle.0.is_null() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("[ERROR] Failed to open process with PID {pid}: Access denied or invalid PID"),
                ));
            }

            werr!(TerminateProcess(process_handle, 0))?;
            werr!(CloseHandle(process_handle))?;
        }

        return Ok(());
        // }

        // TODO: Impl a shutdown message on the daemon so it can gracefully kill itself, else, force ^
    }

    Ok(())
}
