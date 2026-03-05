use std::process::ExitStatus;
use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

use crate::clients::db::Stream;
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

        #[cfg(unix)]
        cmd.process_group(0);

        let child = cmd.spawn().map_err(|e| {
            EngineError::Execution(format!("Failed to spawn workflow process: {}", e))
        })?;

        Ok(child)
    }

    pub async fn monitor(
        mut child_process: Child,
        line_callback: Arc<dyn Fn(Stream, u64, String) + Send + Sync + 'static>,
    ) -> EngineResult<ExecutionOutput> {
        let stdout_callback = Arc::clone(&line_callback);
        let stdout_task = child_process.stdout.take().map(|stdout| {
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                let mut output = String::new();
                let mut seq: u64 = 0;
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!(stream = "stdout", line = %line);
                    stdout_callback(Stream::Stdout, seq, line.clone());
                    seq += 1;
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        });

        let stderr_callback = Arc::clone(&line_callback);
        let stderr_task = child_process.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                let mut output = String::new();
                let mut seq: u64 = 0;
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!(stream = "stderr", line = %line);
                    stderr_callback(Stream::Stderr, seq, line.clone());
                    seq += 1;
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

            // Negate PID to target the entire process group (PGID = PID since
            // we called process_group(0) at spawn time).
            let pgid = Pid::from_raw(-(pid as i32));

            signal::kill(pgid, Signal::SIGTERM).map_err(|e| {
                EngineError::Execution(format!(
                    "Failed to send SIGTERM to process group {} (original pid {}): {}",
                    pgid, pid, e
                ))
            })?;

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            match signal::kill(pgid, None) {
                Ok(_) => {
                    signal::kill(pgid, Signal::SIGKILL).map_err(|e| {
                        EngineError::Execution(format!(
                            "Failed to send SIGKILL to process group {} (original pid {}): {}",
                            pgid, pid, e
                        ))
                    })?;
                },
                Err(Errno::ESRCH) => {
                    info!(pgid = %pgid, pid = %pid, "Process group has already exited");
                },
                Err(e) => {
                    warn!(pgid = %pgid, pid = %pid, error = %e, "Failed to check process group status before SIGKILL");
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
