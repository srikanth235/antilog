#![forbid(unsafe_code)]

//! Antilog on AWS. The core never sees any of this: it is the server's axum `Router`
//! served by Lambda ([`lambda`]). Secrets need no adapter: they come from the function's
//! environment (`antilog_core::secrets::EnvSecrets`).

pub mod deps;
pub mod lambda;
