use std::fs;
use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::process::Command as AsyncCommand;
use tokio::time;

use crate::constants::SOCKET_PATH;

fn get_daemon_process_id() -> io::Result<Option<String>> {
    let cmd = Command::new("ps").arg("-e").output()?;
    let ps_out = String::from_utf8(cmd.stdout).unwrap();

    let Some(process) = ps_out
        .lines()
        .find(|line| line.contains("rustbee-daemon"))
        .map(str::to_owned)
    else {
        return Ok(None);
    };

    let process = process.trim_start();

    let Some(offset) = process.bytes().position(|c| c == b' ') else {
        return Ok(None);
    };

    Ok(Some(process[..offset].to_owned()))
}

// get running process rustbee-daemon
// if running process found:
// - return
//
// spawn rustbee-daemon
// pipe stderr
// wait a sec
// get output status if process exited
// if status is not 0:
// - return err and exit 1
pub async fn launch_daemon() -> io::Result<()> {
    let pid_found = get_daemon_process_id()?;

    if pid_found.is_some() {
        return Ok(());
    }

    let daemon = AsyncCommand::new("rustbee-daemon")
        .stderr(Stdio::piped())
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

// get running process rustbee-daemon
// if the running process is not found:
// - rm SOCKET_FILE
//
// if -f or --force:
// - send SIGKILL to the the process
// - rm SOCKET_FILE
// - return
//
// send SIGINT to the running process for a graceful shutdown
pub fn shutdown_daemon(force: bool) -> io::Result<()> {
    let pid_found = get_daemon_process_id()?;
    if let Some(pid) = pid_found {
        if force {
            Command::new("kill")
                .args(["-s", "KILL", &pid])
                .output()
                .unwrap();

            if fs::exists(SOCKET_PATH)? {
                fs::remove_file(SOCKET_PATH)?;
            }

            return Ok(());
        }

        Command::new("kill")
            .args(["-s", "INT", &pid])
            .output()
            .unwrap();
    } else if fs::exists(SOCKET_PATH)? {
        fs::remove_file(SOCKET_PATH)?;
    }

    Ok(())
}
