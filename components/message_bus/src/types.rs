//! Core types for message bus

use crate::errors::{Error, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use shared_types::BrowserMessage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Trait for sending messages to the bus
pub trait MessageSender: Send + Sync {
    /// Send a message to the message bus
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The bus is shut down
    /// - The channel is disconnected
    fn send(&self, message: BrowserMessage) -> Result<()>;
}

/// Trait for handling messages from the bus
pub trait MessageHandler: Send + Sync {
    /// Handle a received message
    ///
    /// # Errors
    ///
    /// Returns an error if message processing fails
    fn handle(&self, message: BrowserMessage) -> Result<()>;
}

/// Internal sender implementation
struct BusSender {
    sender: Sender<BrowserMessage>,
    shutdown: Arc<AtomicBool>,
}

impl MessageSender for BusSender {
    fn send(&self, message: BrowserMessage) -> Result<()> {
        if self.shutdown.load(Ordering::Relaxed) {
            return Err(Error::NotRunning);
        }

        self.sender
            .send(message)
            .map_err(|e| Error::SendError(e.to_string()))
    }
}

/// State of the message bus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BusState {
    Created,
    Running,
    ShutDown,
}

/// Message bus for component communication
///
/// The message bus provides a channel-based messaging system that allows
/// components to communicate asynchronously. Messages are routed to all
/// registered handlers.
///
/// # Example
///
/// ```
/// use message_bus::{MessageBus, MessageHandler};
/// use shared_types::BrowserMessage;
///
/// struct MyHandler;
/// impl MessageHandler for MyHandler {
///     fn handle(&self, message: BrowserMessage) -> message_bus::Result<()> {
///         println!("Received: {:?}", message);
///         Ok(())
///     }
/// }
///
/// let mut bus = MessageBus::new();
/// bus.register_handler(Box::new(MyHandler));
/// bus.start().unwrap();
///
/// let sender = bus.sender();
/// sender.send(BrowserMessage::Shutdown).unwrap();
///
/// bus.shutdown().unwrap();
/// ```
pub struct MessageBus {
    /// Message sender channel
    sender: Option<Sender<BrowserMessage>>,
    /// Message receiver channel
    receiver: Option<Receiver<BrowserMessage>>,
    /// Registered message handlers
    handlers: Arc<Mutex<Vec<Box<dyn MessageHandler>>>>,
    /// Current state of the bus
    state: BusState,
    /// Worker thread handle
    worker: Option<JoinHandle<()>>,
    /// Shutdown flag shared with senders
    shutdown: Arc<AtomicBool>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        Self {
            sender: None,
            receiver: None,
            handlers: Arc::new(Mutex::new(Vec::new())),
            state: BusState::Created,
            worker: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a message sender for this bus
    ///
    /// # Panics
    ///
    /// Panics if the bus has not been started yet.
    pub fn sender(&self) -> Box<dyn MessageSender> {
        let sender = self
            .sender
            .as_ref()
            .expect("Bus must be started before getting sender")
            .clone();

        Box::new(BusSender {
            sender,
            shutdown: Arc::clone(&self.shutdown),
        })
    }

    /// Register a message handler
    ///
    /// Handlers receive all messages sent to the bus. Multiple handlers
    /// can be registered, and each will receive a copy of every message.
    pub fn register_handler(&mut self, handler: Box<dyn MessageHandler>) {
        self.handlers.lock().unwrap().push(handler);
    }

    /// Start processing messages
    ///
    /// This spawns a worker thread that processes incoming messages and
    /// dispatches them to all registered handlers.
    ///
    /// # Errors
    ///
    /// Returns an error if the bus is already started.
    pub fn start(&mut self) -> Result<()> {
        if self.state == BusState::Running {
            return Err(Error::AlreadyStarted);
        }

        if self.state == BusState::ShutDown {
            return Err(Error::AlreadyShutdown);
        }

        // Create channels
        let (sender, receiver) = unbounded::<BrowserMessage>();
        self.sender = Some(sender);
        self.receiver = Some(receiver);

        // Clone handlers for worker thread
        let handlers = Arc::clone(&self.handlers);
        let receiver = self.receiver.as_ref().unwrap().clone();
        let shutdown = Arc::clone(&self.shutdown);

        // Spawn worker thread
        let worker = thread::spawn(move || {
            loop {
                // Check for shutdown
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                // Try to receive with timeout to periodically check shutdown
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(message) => {
                        let handlers = handlers.lock().unwrap();
                        for handler in handlers.iter() {
                            // Handle each message, but don't stop on errors
                            // This ensures one failing handler doesn't break the bus
                            if let Err(e) = handler.handle(message.clone()) {
                                eprintln!("Handler error: {}", e);
                            }
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // Just continue to check shutdown flag
                        continue;
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        // Channel closed, exit
                        break;
                    }
                }
            }
        });

        self.worker = Some(worker);
        self.state = BusState::Running;

        Ok(())
    }

    /// Shutdown the message bus
    ///
    /// This closes the message channel and waits for the worker thread
    /// to finish processing any remaining messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the bus is not running or already shut down.
    pub fn shutdown(&mut self) -> Result<()> {
        if self.state == BusState::ShutDown {
            return Err(Error::AlreadyShutdown);
        }

        if self.state != BusState::Running {
            return Err(Error::NotRunning);
        }

        // Set shutdown flag first to stop new messages
        self.shutdown.store(true, Ordering::Relaxed);

        // Drop sender to signal channel closure
        self.sender = None;

        // Wait for worker to finish
        if let Some(worker) = self.worker.take() {
            // Join will wait for the thread to complete
            // The thread checks shutdown flag every 100ms
            let _ = worker.join();
        }

        self.state = BusState::ShutDown;
        Ok(())
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

// Ensure MessageBus is Send (but not Sync since it has interior mutability)
// The MessageSender trait objects are Send + Sync for thread safety
