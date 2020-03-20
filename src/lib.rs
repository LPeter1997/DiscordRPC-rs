//! TODO: Introduce

use std::sync;
use sync::atomic::{AtomicBool, Ordering};
use sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::{SystemTime, Duration};
use std::collections::VecDeque;

mod error;
pub use error::*;

pub mod connection;
use connection::*;

mod message;
use message::*;

mod windows;

mod client;
use client::*;

// TODO: Store presence so at reconnect we can re-queue it?

/// The Discord RPC client to communicate with the local Discord server.
#[derive(Debug)]
pub struct DiscordRPC {
    io_proc: IoProcess,
}

impl DiscordRPC {
    /// Creates a new `DiscordRPC` client with the given `Client`.
    fn with_client(client: Client) -> Self {
        Self{
            io_proc: IoProcess::new(client),
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

    /// Starts the client to send and receive messages.
    pub fn start(&mut self) {
        self.io_proc.start();
    }

    /// Sets the `RichPresence` for the Discord server.
    pub fn set_rich_presence(&mut self, rp: Option<RichPresence>) {
        self.io_proc.send(Message::rich_presence(rp));
    }
}

/// Represents a rich-presence description for Discord.
#[derive(Debug, Default, Clone)]
pub struct RichPresence {
    pub state: String,
    pub details: String,
    pub start_timestamp: Option<SystemTime>,
    pub end_timestamp: Option<SystemTime>,
    pub large_image_key: String,
    pub large_image_text: String,
    pub small_image_key: String,
    pub small_image_text: String,
    pub party_id: String,
    pub party_size: usize,
    pub party_max: usize,
    pub match_secret: String,
    pub join_secret: String,
    pub spectate_secret: String,
    pub instance: bool,
}

/// The IO thread manager that basically lets us run in a non-blocking way.
#[derive(Debug)]
struct IoProcess {
    client: Option<Client>,
    keep_running: Arc<AtomicBool>,
    wait_for_io_mux: Arc<Mutex<()>>,
    wait_for_io_cv: Arc<Condvar>,
    send_queue: Arc<Mutex<VecDeque<Message>>>,
    thread_handle: Option<thread::JoinHandle<Client>>,
}

impl IoProcess {
    /// Creates a new `IoProcess` with the given `Client`.
    fn new(client: Client) -> Self {
        let keep_running = Arc::new(AtomicBool::new(true));
        let wait_for_io_mux = Arc::new(Mutex::new(()));
        let wait_for_io_cv = Arc::new(Condvar::new());
        let send_queue = Arc::new(Mutex::new(VecDeque::new()));
        Self{
            client: Some(client),
            keep_running,
            wait_for_io_mux,
            wait_for_io_cv,
            send_queue,
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
        let send_queue = self.send_queue.clone();

        self.thread_handle = Some(thread::spawn(move || {
            const MAX_WAIT: Duration = Duration::from_millis(500);

            let mut last_connect = SystemTime::UNIX_EPOCH;
            Self::update_client(&mut client, &mut last_connect, &send_queue);
            while keep_running.load(Ordering::Relaxed) {
                let lock = wait_for_io_mux.lock().unwrap();
                let _ = wait_for_io_cv.wait_timeout(lock, MAX_WAIT);
                Self::update_client(&mut client, &mut last_connect, &send_queue);
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

    /// Sends a `Message` to the Discord RPC server.
    fn send(&mut self, message: Message) {
        self.send_queue.lock().unwrap().push_back(message);
        self.notify();
    }

    /// Updates the `Client` by doing IO.
    fn update_client(client: &mut Client, last_connect: &mut SystemTime, send_queue: &Arc<Mutex<VecDeque<Message>>>) {
        if !client.is_open() {
            const RECONNECT_DELAY: Duration = Duration::from_millis(1000);

            // Try reconnecting, if there's a second elapsed since the last try
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(*last_connect) {
                if elapsed >= RECONNECT_DELAY {
                    *last_connect = now;
                    client.open();
                }
            }
            return;
        }

        // We are connected

        // Try to read as much as we can
        loop {
            let message = client.read();
            if message.is_none() {
                // Didn't read anything, stop
                break;
            }

            let message = message.unwrap();
            let _evt = message.value("evt");
            let nonce = message.value("nonce");

            if nonce.is_some() {
                // TODO: If evt == "ERROR", report error
            }
            else {
                // TODO:
                // - ACTIVITY_JOIN
                // - ACTIVITY_SPECTATE
                // - ACTIVITY_JOIN_REQUEST
            }
        }

        // Write all pending messages
        {
            let mut send_queue = send_queue.lock().unwrap();
            while let Some(msg) = send_queue.pop_front() {
                if !client.write(msg) {
                    // TODO: Retry?
                }
            }
        }
    }
}

impl Drop for IoProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Returns the current processes ID.
fn pid() -> u32 {
    std::process::id()
}

/// Returns a UUID `String`.
fn nonce() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
