//! TS Dream — Rust server, byte-level port of the TS Online C# server.
//!
//! Module layout mirrors the spec chapters. Foundation layers (protocol,
//! encoding, config, data) are testable without a live database or wire
//! capture; DB-backed and accept-gated parts degrade gracefully.

pub mod config;
pub mod encoding;
pub mod error;
pub mod protocol;
pub mod state;

pub mod data {
    pub mod ini;
    pub mod loader;
    pub mod tables;
    pub mod texps;
}

pub mod db {
    pub mod pool;
}

pub mod server {
    pub mod handler;
    pub mod session;
    pub mod spawn;
}

pub mod web {
    pub mod app;
}

pub mod battle {
    pub mod engine;
}

pub mod harness;