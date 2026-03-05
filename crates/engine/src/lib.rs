#![deny(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::dbg_macro)]
#![forbid(unsafe_code)]

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

pub use engine::Engine;
pub use error::{EngineError, EngineResult};
pub use execution::{ExecutionOutput, ProcessExecutor};
pub use pid_store::PidStore;
pub use runtime::EngineRuntime;
pub use workdir::{WorkdirManager, WorkdirPaths};
