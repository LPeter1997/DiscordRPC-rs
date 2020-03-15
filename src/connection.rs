//! Defines the connection types for the RCP client to use.

use std::marker::Unpin;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;
use crate::Result;

/// A trait that every connection type must implement. This is the main
/// abstraction point for IPC and other communication methods between platforms.
#[async_trait]
pub trait Connection: Sized {
    /// The type that the connection exposes for reading.
    type ReadHalf: AsyncRead + Send + Unpin + 'static;
    /// The type that the connection exposes for writing.
    type WriteHalf: AsyncWrite + Unpin + 'static;

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
    use std::io;
    use std::ops::DerefMut;
    use std::pin::Pin;
    use std::task::{Poll, Context};
    use tokio::fs::{File, OpenOptions};
    use tokio::time;

    /// A wrapper for a `tokio::fs::File` that can be read and written
    /// asynchronously on multiple threads.
    #[derive(Debug, Clone)]
    pub struct AsyncFile(Arc<Mutex<File>>);

    impl AsyncRead for AsyncFile {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8]
        ) -> Poll<io::Result<usize>> {
            Pin::new(self.0.lock().unwrap().deref_mut()).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for AsyncFile {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8]
        ) -> Poll<io::Result<usize>> {
            Pin::new(self.0.lock().unwrap().deref_mut()).poll_write(cx, buf)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context
        ) -> Poll<io::Result<()>> {
            Pin::new(self.0.lock().unwrap().deref_mut()).poll_flush(cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut Context
        ) -> Poll<io::Result<()>> {
            Pin::new(self.0.lock().unwrap().deref_mut()).poll_shutdown(cx)
        }
    }

    /// IPC connection on Windows.
    pub struct IpcConnection(File);

    /// Helper function to asynchronously open a file for reading and writing
    /// with a given optional timeout.
    async fn open_file(address: &str, timeout: Option<Duration>) -> Result<File> {
        let mut opts = OpenOptions::new();
        let fut = opts
            .read(true)
            .write(true)
            .open(&address);
        if let Some(timeout) = timeout {
            let file = time::timeout(timeout, fut);
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
        type ReadHalf = AsyncFile;
        type WriteHalf = AsyncFile;

        async fn connect(index: usize, timeout: Option<Duration>) -> Result<Self> {
            let address = format!(r#"\\.\pipe\discord-ipc-{}"#, index);
            let file = open_file(&address, timeout).await?;
            Ok(Self(file))
        }

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            let file = AsyncFile(Arc::new(Mutex::new(self.0)));
            (file.clone(), file)
        }
    }
}
