//! Defines the connection types for the RCP client to use.

use std::io::{Read, Write};
use crate::Result;

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
pub trait Connection: Read + Write {
    /// Tries to build a connection to the `index`th Dicrord RPC server.
    fn connect(index: usize) -> Result<Self> where Self: Sized;

    /// Returns `true`, if there's anything to read, without blocking the code.
    fn can_read(&mut self) -> Result<bool>;
}

#[cfg(target_os = "windows")]
pub type IpcConnection = crate::windows::NamedPipe;
