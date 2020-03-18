//! Defines the connection trait and types for the RCP client to use.

use std::error;

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
pub trait Connection: Send {
    /// Tries to open a connection to a Discord RPC server. Returns `()`, if the
    /// connection could be established, returns the platform-specific error
    /// description otherwise.
    fn open(&mut self) -> Result<(), Box<dyn error::Error>>;

    /// Returns `true`, if the `Connection` is currently open.
    fn is_open(&self) -> bool;

    /// Closes the `Connection`.
    fn close(&mut self);

    /// Tries to read incoming data from the server, exactly filling the buffer.
    /// The call must be non-blocking, so if there is less available data, the
    /// function simply returns `Ok(false)`. If all bytes were successfully
    /// read, `Ok(true)` is returned. If there was a problem during reading,
    /// the platform-specific error description is returned.
    fn read(&mut self, buffer: &mut [u8]) -> Result<bool, Box<dyn error::Error>>;

    /// Tries to write the bytes to the server. Returns `()`, if all bytes were
    /// successfully written, the platform-specific error description otherwise.
    fn write(&mut self, buffer: &[u8]) -> Result<(), Box<dyn error::Error>>;
}

#[cfg(target_os = "windows")]
pub type IpcConnection = crate::windows::NamedPipe;
