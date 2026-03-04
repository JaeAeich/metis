#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::dbg_macro)]
#![forbid(unsafe_code)]

pub mod cli;
pub mod clients;
pub mod command;
pub mod config;
pub mod context;
pub mod dry_run;
pub mod engine;
pub mod error;
pub mod execution;
pub mod models;
pub mod pid_store;
pub mod runtime;
pub mod server;
pub mod workdir;

pub use cli::run_cli;
pub use engine::Engine;
pub use error::{EngineError, EngineResult};
pub use execution::{ExecutionOutput, ProcessExecutor};
pub use pid_store::PidStore;
pub use runtime::EngineRuntime;
pub use workdir::{WorkdirManager, WorkdirPaths};

/// Bootstrap the engine with CLI argument parsing.
/// This function parses command-line arguments and dispatches to either
/// `run` (single workflow execution) or `server` (long-running server) mode.
///
/// # Example
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> engine::Result<()> {
///     engine::bootstrap::<MyEngine>().await
/// }
/// ```
pub async fn bootstrap<E: Engine + 'static>() -> EngineResult<()> {
    let engine = E::new();
    run_cli(engine).await
}
