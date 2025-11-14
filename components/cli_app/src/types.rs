//! Type definitions for the CLI application

use crate::errors::Result;
use adblock_engine::AdBlockEngine;
use browser_core::BrowserEngine;
use browser_shell::BrowserShell;
use config_manager::Config;
use message_bus::MessageBus;
use network_stack::NetworkStack;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Main browser application struct
///
/// This struct wires together all components of the FrankenBrowser
/// and manages their lifecycle.
pub struct BrowserApp {
    /// Tokio async runtime
    /// Note: Not directly accessed but required for the shell's operation
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    /// Message bus for component communication
    message_bus: MessageBus,
    /// Ad blocking engine
    adblock: AdBlockEngine,
    /// Browser core engine (contains network stack)
    /// Note: Initialized during construction but not directly accessed in run()
    #[allow(dead_code)]
    browser_core: BrowserEngine,
    /// Browser shell (UI)
    shell: BrowserShell,
}

impl BrowserApp {
    /// Create a new browser application
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for all components
    ///
    /// # Returns
    ///
    /// Returns a `Result<BrowserApp>` with the initialized application.
    ///
    /// # Errors
    ///
    /// Returns an error if any component fails to initialize.
    pub fn new(config: Config) -> Result<Self> {
        // Create tokio runtime
        let runtime = Arc::new(
            Runtime::new().map_err(|e| anyhow::anyhow!("Failed to create runtime: {}", e))?,
        );

        // Create message bus
        let mut message_bus = MessageBus::new();

        // Start message bus to get senders
        message_bus.start()?;

        // Create network stack with its own sender
        let network = NetworkStack::new(config.network_config(), message_bus.sender())?;

        // Create adblock engine with its own sender
        let adblock = AdBlockEngine::new(config.adblock_config(), message_bus.sender())?;

        // Create browser core engine
        // Note: BrowserEngine takes NetworkStack by value, so we create it here
        let browser_core = BrowserEngine::new(config.clone(), network, message_bus.sender())?;

        // Create browser shell with its own sender
        let shell =
            BrowserShell::new(config.shell_config(), message_bus.sender(), runtime.clone())?;

        Ok(Self {
            runtime,
            message_bus,
            adblock,
            browser_core,
            shell,
        })
    }

    /// Run the browser application
    ///
    /// This method starts all components and runs the browser shell event loop.
    /// It blocks until the browser is closed by the user.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error if any component fails during initialization or runtime.
    pub fn run(mut self) -> Result<()> {
        // Initialize components
        // Note: NetworkStack is owned by BrowserEngine and initialized there
        self.adblock.initialize()?;

        // Run the browser shell (blocks until exit)
        self.shell.run()?;

        // Cleanup: shutdown message bus
        self.message_bus.shutdown()?;

        Ok(())
    }
}
