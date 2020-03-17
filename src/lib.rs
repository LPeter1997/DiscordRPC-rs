//! TODO: Introduce

use std::sync;
use sync::atomic::{AtomicBool, Ordering};
use sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;

pub mod connection;
use connection::*;

mod message;
use message::*;

mod windows;
use windows::*;

mod client;
use client::*;

/// The Discord RPC client to communicate with the local Discord server.
pub struct DiscordRPC {
    client: Client,
}

impl DiscordRPC {
    /// Creates a new `DiscordRPC` client with the given `Client`.
    fn with_client(client: Client) -> Self {
        Self{
            client,
        }
    }

    /// Creates a new `DiscordRPC` client with the given `Connection` and
    /// application ID.
    pub fn with_connection<C: Connection>(conn: C, app_id: &str) -> Self where C: 'static {
        Self::with_client(Client::with_connection(conn, app_id))
    }

    /// Creates a new `DiscordRPC` client with the given application ID and the
    /// default connection type.
    pub fn new(app_id: &str) -> Self {
        Self::with_client(Client::new(app_id))
    }
}

/// The IO thread manager.
struct IoProcess {
    client: Option<Client>,
    keep_running: Arc<AtomicBool>,
    wait_for_io_mux: Arc<Mutex<()>>,
    wait_for_io_cv: Arc<Condvar>,
    thread_handle: Option<thread::JoinHandle<Client>>,
}

impl IoProcess {
    /// Creates a new `IoProcess` with the given `Client`.
    fn new(client: Client) -> Self {
        let keep_running = Arc::new(AtomicBool::new(true));
        let wait_for_io_mux = Arc::new(Mutex::new(()));
        let wait_for_io_cv = Arc::new(Condvar::new());
        Self {
            client: Some(client),
            keep_running,
            wait_for_io_mux,
            wait_for_io_cv,
            thread_handle: None,
        }
    }

    /// Starts the IO thread.
    fn start(&mut self) {
        if self.thread_handle.is_some() {
            return;
        }

        self.keep_running.store(true, Ordering::Relaxed);

        let mut client = self.client.take().unwrap();
        let keep_running = self.keep_running.clone();
        let wait_for_io_mux = self.wait_for_io_mux.clone();
        let wait_for_io_cv = self.wait_for_io_cv.clone();

        self.thread_handle = Some(thread::spawn(move || {
            const MAX_WAIT: Duration = Duration::from_millis(500);

            Self::update_client(&mut client);
            while keep_running.load(Ordering::Relaxed) {
                let lock = wait_for_io_mux.lock().unwrap();
                let _ = wait_for_io_cv.wait_timeout(lock, MAX_WAIT);
                Self::update_client(&mut client);
            }

            client
        }));
    }

    /// Stops the IO thread.
    fn stop(&mut self) {
        if self.thread_handle.is_none() {
            return;
        }

        self.keep_running.store(false, Ordering::Relaxed);
        self.notify();
        self.client = Some(self.thread_handle.take().unwrap().join().unwrap());
    }

    /// Notifies IO activity.
    fn notify(&mut self) {
        self.wait_for_io_cv.notify_all();
    }

    /// Updates the `Client` by doing IO.
    fn update_client(client: &mut Client) {
        unimplemented!();
    }
}

impl Drop for IoProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
