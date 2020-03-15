//! Defines the connection types for the RCP client to use.

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;
use crate::Result;

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
#[async_trait]
pub trait Connection: Sized {
    /// The type that the connection exposes for reading.
    type ReadHalf: AsyncRead;
    /// The type that the connection exposes for writing.
    type WriteHalf: AsyncWrite;

    /// Tries to build a connection to the `index`th Dicrord RPC server. An
    /// optional timeout can be given.
    async fn connect(index: usize, timeout: Option<Duration>) -> Result<Self>;

    /// Splits this connection into a `ReadHalf` and `WriteHalf`.
    fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

/// IPC connection for the platform.
#[cfg(target_os = "windows")]
pub type IpcConnection = windows::IpcConnection;

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use tokio::fs::{File, OpenOptions};

    /// IPC connection on Windows.
    pub struct IpcConnection {
        read: File,
        write: File,
    }

    /// Helper function to asynchronously open a file for reading or writing
    /// with a given timeout.
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

    #[async_trait]
    impl Connection for IpcConnection {
        type ReadHalf = File;
        type WriteHalf = File;

        async fn connect(index: usize, timeout: Option<Duration>) -> Result<Self> {
            let address = format!(r#"\\.\pipe\discord-ipc-{}"#, index);
            let read = open_file(&address, true, false, timeout).await?;
            let write = open_file(&address, false, true, timeout).await?;
            Ok(Self{ read, write })
        }

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (self.read, self.write)
        }
    }
}
