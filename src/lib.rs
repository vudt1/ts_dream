//! TS Dream — Rust server, byte-level port of the TS Online server.
//!
//! Module layout mirrors the spec chapters. Foundation layers (protocol,
//! encoding, config, data) are testable without a live database or wire
//! capture; DB-backed and accept-gated parts degrade gracefully.

pub mod config;
pub mod encoding;
pub mod error;
pub mod eve;
pub mod protocol;
pub mod state;

pub mod data {
    pub mod ini;
    pub mod loader;
    pub mod loaders;
    pub mod reader;
    pub mod tables;
    pub mod texps;
}

pub mod db {
    pub mod accounts;
    pub mod catalog;
    pub mod domain;
    pub mod item_code;
    pub mod modern;
    pub mod persist;
    pub mod pool;
}

pub mod server {
    pub mod auto_save;
    pub mod character_sheet;
    pub mod dispatcher;
    pub mod gm;
    pub mod handlers;
    pub mod inventory;
    pub mod map_drops;
    pub mod pet_box;
    pub mod player_state;
    pub mod response;
    pub mod session;
    pub mod spawn;
    pub mod trade_system;
}

pub mod web {
    pub mod app;
    pub mod server_control;
}

pub mod battle {
    pub mod construction;
    pub mod damage;
    pub mod engine;
    pub mod manager;
    pub mod mobile_damage;
    pub mod npc_world;
    pub mod packets;
    pub mod rng;
    pub mod runner;
    pub mod service;
    pub mod targeting;
}

pub mod harness;
