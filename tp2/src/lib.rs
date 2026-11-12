//! TP2 receives the Nano's semicolon-separated CSV and stores validated samples
//! in the existing MariaDB `measurements` table. See the crate README for setup.

pub mod config;
pub mod database;
pub mod framing;
pub mod measurement;
pub mod service;
pub mod simulator;
