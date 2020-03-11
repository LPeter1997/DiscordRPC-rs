//! Defines the connection types for the RCP client to use.

use std::marker::Send;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;
use crate::Result;

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
pub trait Connection: Sized {
    /// The type that the connection exposes for reading.
    type ReadHalf: AsyncRead + Unpin + Send + 'static;
    /// The type that the connection exposes for writing.
    type WriteHalf: AsyncWrite + Unpin;

    /// Tries to build a connection to the `index`th Dicrord RPC server. An
    /// optional timeout can be given.
    fn connect(index: usize, timeout: Option<Duration>) -> Result<Self>;

    /// Splits this connection into a `ReadHalf` and `WriteHalf`.
    fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

#[cfg(target_os = "windows")]
pub type IpcConnection = windows::IpcConnection;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use futures::executor::block_on;
    use tokio::fs::{File, OpenOptions};

    /// IPC connection on Windows.
    pub struct IpcConnection {
        read: File,
        write: File,
    }

    async fn open_file(address: &str, r: bool, w: bool, timeout: Option<Duration>) -> Result<File> {
        let mut opts = OpenOptions::new();
        opts.read(r).write(w);
        let fut = opts.open(&address);
        if let Some(timeout) = timeout {
            let file = tokio::time::timeout(timeout, fut);
            let file = file.await??;
            Ok(file)
        }
        else {
            let file = fut.await?;
            Ok(file)
        }
    }

    impl Connection for IpcConnection {
        type ReadHalf = File;
        type WriteHalf = File;

        // TODO: Make this async somehow?
        fn connect(index: usize, timeout: Option<Duration>) -> Result<Self> {
            let address = format!(r#"\\.\pipe\discord-ipc-{}"#, index);
            let read = block_on(open_file(&address, true, false, timeout))?;
            let write = block_on(open_file(&address, false, true, timeout))?;
            Ok(Self{ read, write })
        }

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (self.read, self.write)
        }
    }
}
