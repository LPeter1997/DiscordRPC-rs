//! Defines the connection types for the RCP client to use.

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
pub trait Connection: Send {
    /// Tries to open a connection to a Discord RPC server. Returns `true`, if
    /// the connection could be established.
    fn open(&mut self) -> bool;

    /// Returns `true`, if the `Connection` is currently open.
    fn is_open(&self) -> bool;

    /// Closes the `Connection`.
    fn close(&mut self);

    /// Tries to read incoming data from the server, exactly filling the buffer.
    /// The call must be non-blocking, so if there is less available data, the
    /// function simply returns `false`. If all bytes were successfully read,
    /// `true` is returned.
    fn read(&mut self, buffer: &mut [u8]) -> bool;

    /// Tries to write the bytes to the server. Returns `true`, if all bytes
    /// were successfully written, `false` otherwise.
    fn write(&mut self, buffer: &[u8]) -> bool;
}

#[cfg(target_os = "windows")]
pub type IpcConnection = crate::windows::NamedPipe;
