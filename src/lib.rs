//! TODO: Introduce

mod error;
pub use error::*;

pub mod connection;
mod message;
pub mod client;

pub use client::Client;
