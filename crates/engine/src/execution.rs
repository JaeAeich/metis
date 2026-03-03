use std::process::ExitStatus;

use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

use crate::error::{EngineError, EngineResult};
use crate::models::CommandInfo;

pub struct ExecutionOutput {
    pub exit_status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub struct ProcessExecutor;

impl ProcessExecutor {
    pub async fn execute(command_info: &CommandInfo) -> EngineResult<Child> {
        let mut cmd = if command_info.env_vars.is_empty() {
            Command::new("sh")
        } else {
            let mut c = Command::new("sh");
            for (key, value) in &command_info.env_vars {
                c.env(key, value);
            }
            c
        };

        cmd.arg("-c")
            .arg(&command_info.command)
            .current_dir(&command_info.workdir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| {
            EngineError::Execution(format!("Failed to spawn workflow process: {}", e))
        })?;

        Ok(child)
    }

    pub async fn monitor(mut child_process: Child) -> EngineResult<ExecutionOutput> {
        let stdout_task = child_process.stdout.take().map(|stdout| {
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!(stream = "stdout", line = %line);
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        });

        let stderr_task = child_process.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                let mut output = String::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!(stream = "stderr", line = %line);
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        });

        let status = child_process.wait().await.map_err(|e| {
            EngineError::Execution(format!(
                "Failed while waiting for workflow process to complete: {}",
                e
            ))
        })?;

        let stdout = if let Some(task) = stdout_task {
            match task.await {
                Ok(output) => output,
                Err(e) => {
                    warn!(error = %e, "Failed to join stdout reader task");
                    String::new()
                },
            }
        } else {
            String::new()
        };

        let stderr = if let Some(task) = stderr_task {
            match task.await {
                Ok(output) => output,
                Err(e) => {
                    warn!(error = %e, "Failed to join stderr reader task");
                    String::new()
                },
            }
        } else {
            String::new()
        };

        Ok(ExecutionOutput { exit_status: status, stdout, stderr })
    }

    pub async fn cancel(pid: u32) -> EngineResult<()> {
        #[cfg(unix)]
        {
            use nix::errno::Errno;
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;

            let pid = Pid::from_raw(pid as i32);

            signal::kill(pid, Signal::SIGTERM).map_err(|e| {
                EngineError::Execution(format!("Failed to send SIGTERM to process {}: {}", pid, e))
            })?;

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            match signal::kill(pid, None) {
                Ok(_) => {
                    signal::kill(pid, Signal::SIGKILL).map_err(|e| {
                        EngineError::Execution(format!(
                            "Failed to send SIGKILL to process {}: {}",
                            pid, e
                        ))
                    })?;
                },
                Err(Errno::ESRCH) => {
                    info!(pid = %pid, "Process has already exited");
                },
                Err(e) => {
                    warn!(pid = %pid, error = %e, "Failed to check process status before SIGKILL");
                },
            }
        }

        #[cfg(not(unix))]
        {
            return Err(EngineError::Execution(
                "Process cancellation not implemented for this platform".to_string(),
            ));
        }

        Ok(())
    }
}
