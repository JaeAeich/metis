// use nix::sys::signal::{Signal, kill};
// use nix::unistd::Pid;
// use std::convert::TryFrom;
// use std::error::Error;
// use std::process::Stdio;
// use tokio::io::{AsyncBufReadExt, BufReader};
// use tokio::process::Command;
// use tracing::{debug, error, info};

// /// Result of a process execution
// #[derive(Debug)]
// pub struct ProcessResult {
//     pub exit_code: i32,
//     pub stdout: String,
//     pub stderr: String,
// }

// /// Spawn a process and wait for completion, capturing output
// pub async fn spawn_and_wait(
//     command: Vec<String>,
//     work_dir: Option<&str>,
// ) -> Result<ProcessResult, Box<dyn Error>> {
//     if command.is_empty() {
//         return Err("Command cannot be empty".into());
//     }

//     let program = &command[0];
//     let args = &command[1..];

//     info!(
//         program = %program,
//         args = ?args,
//         work_dir = ?work_dir,
//         "Spawning process"
//     );

//     let mut cmd = Command::new(program);
//     cmd.args(args)
//         .stdout(Stdio::piped())
//         .stderr(Stdio::piped())
//         .stdin(Stdio::null());

//     if let Some(dir) = work_dir {
//         cmd.current_dir(dir);
//     }

//     let mut child = cmd.spawn().map_err(|e| {
//         error!(error = %e, "Failed to spawn process");
//         e
//     })?;

//     let pid = child.id().unwrap_or(0);
//     info!(pid = pid, "Process started");

//     // Capture stdout and stderr
//     let stdout = child.stdout.take().expect("Failed to capture stdout");
//     let stderr = child.stderr.take().expect("Failed to capture stderr");

//     let stdout_reader = BufReader::new(stdout);
//     let stderr_reader = BufReader::new(stderr);

//     // Read output asynchronously
//     let stdout_handle = tokio::spawn(async move {
//         let mut lines = stdout_reader.lines();
//         let mut output = String::new();
//         while let Ok(Some(line)) = lines.next_line().await {
//             debug!(stream = "stdout", line = %line);
//             output.push_str(&line);
//             output.push('\n');
//         }
//         output
//     });

//     let stderr_handle = tokio::spawn(async move {
//         let mut lines = stderr_reader.lines();
//         let mut output = String::new();
//         while let Ok(Some(line)) = lines.next_line().await {
//             debug!(stream = "stderr", line = %line);
//             output.push_str(&line);
//             output.push('\n');
//         }
//         output
//     });

//     // Wait for process to complete
//     let status = child.wait().await.map_err(|e| {
//         error!(error = %e, "Failed to wait for process");
//         e
//     })?;

//     let stdout = stdout_handle.await.unwrap_or_default();
//     let stderr = stderr_handle.await.unwrap_or_default();

//     let exit_code = status.code().unwrap_or(-1);

//     info!(pid = pid, exit_code = exit_code, "Process completed");

//     Ok(ProcessResult {
//         exit_code,
//         stdout,
//         stderr,
//     })
// }

// /// Spawn a process and return immediately with its PID
// /// Useful for long-running workflows where you want to track the PID
// pub async fn spawn_detached(
//     command: Vec<String>,
//     work_dir: Option<&str>,
// ) -> Result<u32, Box<dyn Error>> {
//     if command.is_empty() {
//         return Err("Command cannot be empty".into());
//     }

//     let program = &command[0];
//     let args = &command[1..];

//     info!(
//         program = %program,
//         args = ?args,
//         work_dir = ?work_dir,
//         "Spawning detached process"
//     );

//     let mut cmd = Command::new(program);
//     cmd.args(args)
//         .stdout(Stdio::null())
//         .stderr(Stdio::null())
//         .stdin(Stdio::null());

//     if let Some(dir) = work_dir {
//         cmd.current_dir(dir);
//     }

//     let child = cmd.spawn().map_err(|e| {
//         error!(error = %e, "Failed to spawn process");
//         e
//     })?;

//     let pid = child
//         .id()
//         .ok_or_else(|| "Failed to get process ID".to_string())?;

//     info!(pid = pid, "Process spawned successfully");

//     // Don't wait for the child - let it run independently
//     // The process will continue running even if we drop the Child handle
//     std::mem::forget(child);

//     Ok(pid)
// }

// /// Check if a process is still running
// #[cfg(unix)]
// pub fn is_process_running(pid: i32) -> bool {
//     let sig = Signal::try_from(0).unwrap(); // signal 0 = "no-op check"
//     matches!(
//         kill(Pid::from_raw(pid), sig),
//         Ok(_) | Err(nix::Error::EPERM)
//     )
// }

// #[cfg(not(unix))]
// pub fn is_process_running(_pid: u32) -> bool {
//     // Platform-specific implementation needed
//     false
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_spawn_and_wait_success() {
//         let result = spawn_and_wait(vec!["echo".to_string(), "hello".to_string()], None).await;
//         assert!(result.is_ok());
//         let output = result.unwrap();
//         assert_eq!(output.exit_code, 0);
//         assert!(output.stdout.contains("hello"));
//     }

//     #[tokio::test]
//     async fn test_spawn_and_wait_failure() {
//         let result = spawn_and_wait(vec!["false".to_string()], None).await;
//         assert!(result.is_ok());
//         let output = result.unwrap();
//         assert_ne!(output.exit_code, 0);
//     }

//     #[tokio::test]
//     async fn test_spawn_detached() {
//         let result = spawn_detached(vec!["sleep".to_string(), "0.1".to_string()], None).await;
//         assert!(result.is_ok());
//         let pid = result.unwrap();
//         assert!(pid > 0);
//     }

//     #[tokio::test]
//     async fn test_empty_command() {
//         let result = spawn_and_wait(vec![], None).await;
//         assert!(result.is_err());
//     }
// }
