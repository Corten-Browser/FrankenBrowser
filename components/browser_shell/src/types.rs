//! Core types for browser shell

use crate::errors::{Error, Result};
use config_manager::ShellConfig;
use message_bus::MessageSender;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

// WRY and tao imports for GUI mode
#[cfg(feature = "gui")]
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    dpi::LogicalSize,
};

#[cfg(feature = "gui")]
use wry::WebViewBuilder;

/// Represents a browser tab with its state
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique identifier for the tab
    pub id: u32,
    /// Current URL loaded in the tab
    pub url: Option<String>,
    /// Tab title
    pub title: String,
    /// Whether the tab is currently loading
    pub is_loading: bool,
}

impl Tab {
    /// Create a new tab with the given ID
    pub fn new(id: u32) -> Self {
        Self {
            id,
            url: None,
            title: "New Tab".to_string(),
            is_loading: false,
        }
    }
}

/// Browser shell that manages windows and tabs
pub struct BrowserShell {
    /// Configuration for the shell
    /// Note: In headless mode, this is stored but not actively used for window rendering
    #[allow(dead_code)]
    config: ShellConfig,
    /// Message sender for component communication
    message_sender: Box<dyn MessageSender>,
    /// Async runtime
    /// Note: Reserved for future async window operations in GUI mode
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    /// All open tabs
    tabs: HashMap<u32, Tab>,
    /// Currently active tab ID
    active_tab: Option<u32>,
    /// Next available tab ID
    next_tab_id: u32,
    /// Event loop for GUI mode (Option because we take ownership when running)
    #[cfg(feature = "gui")]
    event_loop: Option<EventLoop<()>>,
    /// Window handle for GUI mode
    #[cfg(feature = "gui")]
    #[allow(dead_code)]
    window: Option<Window>,
    /// WebView instance for GUI mode
    #[cfg(feature = "gui")]
    #[allow(dead_code)]
    webview: Option<wry::WebView>,
}

impl BrowserShell {
    /// Create a new BrowserShell
    ///
    /// # Arguments
    ///
    /// * `config` - Shell configuration
    /// * `sender` - Message sender for component communication
    /// * `runtime` - Tokio async runtime
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails
    pub fn new(
        config: ShellConfig,
        sender: Box<dyn MessageSender>,
        runtime: Arc<Runtime>,
    ) -> Result<Self> {
        #[cfg(feature = "gui")]
        {
            // Create event loop and window for GUI mode using tao
            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .with_title("Frankenstein Browser")
                .with_inner_size(LogicalSize::new(1280, 720))
                .build(&event_loop)
                .map_err(|e| Error::Initialization(format!("Failed to create window: {}", e)))?;

            // Create WebView using wry 0.35
            // We need to use a reference to the window for WebViewBuilder
            let webview = {
                // Use a local scope to create webview with borrowed window
                let wb = WebViewBuilder::new(&window);
                wb.with_url(&config.homepage)
                    .map_err(|e| Error::Initialization(format!("Failed to set URL: {}", e)))?
                    .build()
                    .map_err(|e| Error::Initialization(format!("Failed to build webview: {}", e)))?
            };

            Ok(Self {
                config,
                message_sender: sender,
                runtime,
                tabs: HashMap::new(),
                active_tab: None,
                next_tab_id: 1,
                event_loop: Some(event_loop),
                window: Some(window),
                webview: Some(webview),
            })
        }

        #[cfg(not(feature = "gui"))]
        {
            // Headless mode - no window creation
            Ok(Self {
                config,
                message_sender: sender,
                runtime,
                tabs: HashMap::new(),
                active_tab: None,
                next_tab_id: 1,
            })
        }
    }

    /// Check if this shell has a window (GUI mode only)
    ///
    /// # Returns
    ///
    /// True if running in GUI mode with a window, false otherwise
    #[allow(dead_code)]
    pub fn has_window(&self) -> bool {
        #[cfg(feature = "gui")]
        {
            self.window.is_some()
        }
        #[cfg(not(feature = "gui"))]
        {
            false
        }
    }

    /// Run the browser shell event loop
    ///
    /// This starts the window management and event processing.
    /// In a headless environment, this is a stub.
    ///
    /// # Errors
    ///
    /// Returns an error if the event loop fails to start
    pub fn run(&mut self) -> Result<()> {
        #[cfg(feature = "gui")]
        {
            // Take ownership of event loop (it can only be run once)
            if let Some(event_loop) = self.event_loop.take() {
                event_loop.run(move |event, _, control_flow| {
                    *control_flow = ControlFlow::Wait;

                    match event {
                        Event::WindowEvent {
                            event: WindowEvent::CloseRequested,
                            ..
                        } => {
                            *control_flow = ControlFlow::Exit;
                        }
                        _ => {}
                    }
                });
            }
            Ok(())
        }

        #[cfg(not(feature = "gui"))]
        {
            // In headless environment, this is a stub
            Ok(())
        }
    }

    /// Create a new tab
    ///
    /// # Returns
    ///
    /// The ID of the newly created tab
    ///
    /// # Errors
    ///
    /// Returns an error if tab creation fails
    pub fn create_tab(&mut self) -> Result<u32> {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = Tab::new(tab_id);
        self.tabs.insert(tab_id, tab);

        // Set as active tab if it's the first tab
        if self.active_tab.is_none() {
            self.active_tab = Some(tab_id);
        }

        // Send CreateTab message to message bus
        use shared_types::BrowserMessage;
        let _ = self.message_sender.send(BrowserMessage::CreateTab {
            parent_window: 0, // Default window ID
        });

        Ok(tab_id)
    }

    /// Close a tab
    ///
    /// # Arguments
    ///
    /// * `tab_id` - The ID of the tab to close
    ///
    /// # Errors
    ///
    /// Returns an error if the tab doesn't exist
    pub fn close_tab(&mut self, tab_id: u32) -> Result<()> {
        if !self.tabs.contains_key(&tab_id) {
            return Err(Error::TabNotFound(tab_id));
        }

        self.tabs.remove(&tab_id);

        // If we closed the active tab, switch to another tab
        if self.active_tab == Some(tab_id) {
            self.active_tab = self.tabs.keys().next().copied();
        }

        // Send CloseTab message
        use shared_types::BrowserMessage;
        let _ = self
            .message_sender
            .send(BrowserMessage::CloseTab { tab_id });

        Ok(())
    }

    /// Switch to a different tab
    ///
    /// # Arguments
    ///
    /// * `tab_id` - The ID of the tab to switch to
    ///
    /// # Errors
    ///
    /// Returns an error if the tab doesn't exist
    pub fn switch_to_tab(&mut self, tab_id: u32) -> Result<()> {
        if !self.tabs.contains_key(&tab_id) {
            return Err(Error::TabNotFound(tab_id));
        }

        self.active_tab = Some(tab_id);

        // Send SwitchTab message
        use shared_types::BrowserMessage;
        let _ = self
            .message_sender
            .send(BrowserMessage::SwitchTab { tab_id });

        Ok(())
    }

    /// Get the number of open tabs
    ///
    /// # Returns
    ///
    /// The count of currently open tabs
    pub fn get_tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get the active tab ID
    ///
    /// # Returns
    ///
    /// The ID of the currently active tab, or None if no tabs are open
    pub fn get_active_tab(&self) -> Option<u32> {
        self.active_tab
    }

    /// Get a reference to a tab by ID
    ///
    /// # Arguments
    ///
    /// * `tab_id` - The ID of the tab to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the tab, or None if the tab doesn't exist
    pub fn get_tab(&self, tab_id: u32) -> Option<&Tab> {
        self.tabs.get(&tab_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use message_bus::MessageBus;

    fn create_test_shell() -> BrowserShell {
        let config = ShellConfig {
            homepage: "https://www.example.com".to_string(),
            enable_devtools: true,
            theme: "light".to_string(),
            default_zoom: 1.0,
        };

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let runtime = Arc::new(Runtime::new().unwrap());

        BrowserShell::new(config, sender, runtime).unwrap()
    }

    // ========================================
    // RED PHASE: Tests for BrowserShell::new()
    // ========================================

    #[test]
    fn test_browser_shell_new() {
        let shell = create_test_shell();
        assert_eq!(shell.get_tab_count(), 0);
        assert_eq!(shell.get_active_tab(), None);
    }

    #[test]
    fn test_browser_shell_new_with_config() {
        let config = ShellConfig {
            homepage: "https://www.test.com".to_string(),
            enable_devtools: false,
            theme: "dark".to_string(),
            default_zoom: 1.5,
        };

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        let runtime = Arc::new(Runtime::new().unwrap());

        let shell = BrowserShell::new(config.clone(), sender, runtime).unwrap();
        assert_eq!(shell.config.homepage, "https://www.test.com");
        assert_eq!(shell.config.enable_devtools, false);
        assert_eq!(shell.config.theme, "dark");
        assert_eq!(shell.config.default_zoom, 1.5);
    }

    // ========================================
    // RED PHASE: Tests for tab management
    // ========================================

    #[test]
    fn test_create_tab() {
        let mut shell = create_test_shell();
        let tab_id = shell.create_tab().unwrap();

        assert_eq!(tab_id, 1);
        assert_eq!(shell.get_tab_count(), 1);
        assert_eq!(shell.get_active_tab(), Some(1));
    }

    #[test]
    fn test_create_multiple_tabs() {
        let mut shell = create_test_shell();

        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();
        let tab3 = shell.create_tab().unwrap();

        assert_eq!(tab1, 1);
        assert_eq!(tab2, 2);
        assert_eq!(tab3, 3);
        assert_eq!(shell.get_tab_count(), 3);
        assert_eq!(shell.get_active_tab(), Some(1)); // First tab is active
    }

    #[test]
    fn test_close_tab() {
        let mut shell = create_test_shell();
        let tab_id = shell.create_tab().unwrap();

        shell.close_tab(tab_id).unwrap();
        assert_eq!(shell.get_tab_count(), 0);
        assert_eq!(shell.get_active_tab(), None);
    }

    #[test]
    fn test_close_tab_not_found() {
        let mut shell = create_test_shell();
        let result = shell.close_tab(999);

        assert!(result.is_err());
        match result {
            Err(Error::TabNotFound(id)) => assert_eq!(id, 999),
            _ => panic!("Expected TabNotFound error"),
        }
    }

    #[test]
    fn test_close_middle_tab() {
        let mut shell = create_test_shell();

        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();
        let tab3 = shell.create_tab().unwrap();

        shell.close_tab(tab2).unwrap();

        assert_eq!(shell.get_tab_count(), 2);
        assert!(shell.get_tab(tab1).is_some());
        assert!(shell.get_tab(tab2).is_none());
        assert!(shell.get_tab(tab3).is_some());
    }

    #[test]
    fn test_close_active_tab_switches_to_another() {
        let mut shell = create_test_shell();

        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();

        // Tab1 is active
        assert_eq!(shell.get_active_tab(), Some(tab1));

        // Close the active tab
        shell.close_tab(tab1).unwrap();

        // Should switch to the other tab
        assert_eq!(shell.get_active_tab(), Some(tab2));
    }

    #[test]
    fn test_switch_to_tab() {
        let mut shell = create_test_shell();

        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();

        assert_eq!(shell.get_active_tab(), Some(tab1));

        shell.switch_to_tab(tab2).unwrap();
        assert_eq!(shell.get_active_tab(), Some(tab2));
    }

    #[test]
    fn test_switch_to_tab_not_found() {
        let mut shell = create_test_shell();
        shell.create_tab().unwrap();

        let result = shell.switch_to_tab(999);

        assert!(result.is_err());
        match result {
            Err(Error::TabNotFound(id)) => assert_eq!(id, 999),
            _ => panic!("Expected TabNotFound error"),
        }
    }

    #[test]
    fn test_get_tab_count() {
        let mut shell = create_test_shell();

        assert_eq!(shell.get_tab_count(), 0);

        shell.create_tab().unwrap();
        assert_eq!(shell.get_tab_count(), 1);

        shell.create_tab().unwrap();
        assert_eq!(shell.get_tab_count(), 2);

        shell.close_tab(1).unwrap();
        assert_eq!(shell.get_tab_count(), 1);
    }

    #[test]
    fn test_get_tab() {
        let mut shell = create_test_shell();
        let tab_id = shell.create_tab().unwrap();

        let tab = shell.get_tab(tab_id);
        assert!(tab.is_some());

        let tab = tab.unwrap();
        assert_eq!(tab.id, tab_id);
        assert_eq!(tab.title, "New Tab");
    }

    #[test]
    fn test_get_tab_not_found() {
        let shell = create_test_shell();
        let tab = shell.get_tab(999);
        assert!(tab.is_none());
    }

    #[test]
    fn test_run() {
        let mut shell = create_test_shell();
        // In headless mode, run() should succeed but do nothing
        let result = shell.run();
        assert!(result.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for WRY window integration
    // ========================================

    #[test]
    #[cfg(all(feature = "gui", target_os = "linux"))]
    fn test_browser_shell_creates_window() {
        // Test that BrowserShell can create an actual WRY window
        // This test requires DISPLAY=:99 (Xvfb) to be running
        std::env::set_var("DISPLAY", ":99");

        let config = ShellConfig {
            homepage: "https://www.example.com".to_string(),
            enable_devtools: true,
            theme: "light".to_string(),
            default_zoom: 1.0,
        };

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        let runtime = Arc::new(Runtime::new().unwrap());

        // This should create an actual window with event loop
        let result = BrowserShell::new(config, sender, runtime);
        assert!(result.is_ok(), "Failed to create BrowserShell with GUI: {:?}", result.err());

        bus.shutdown().unwrap();
    }

    #[test]
    #[cfg(all(feature = "gui", target_os = "linux"))]
    fn test_browser_shell_window_has_event_loop() {
        // Test that BrowserShell holds an event loop
        std::env::set_var("DISPLAY", ":99");

        let config = ShellConfig {
            homepage: "https://www.example.com".to_string(),
            enable_devtools: true,
            theme: "light".to_string(),
            default_zoom: 1.0,
        };

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        let runtime = Arc::new(Runtime::new().unwrap());

        let shell = BrowserShell::new(config, sender, runtime).unwrap();

        // Shell should have event loop ready to run
        // We can't test run() directly as it blocks, but we can verify creation succeeded
        assert!(shell.has_window(), "BrowserShell should have window in GUI mode");

        bus.shutdown().unwrap();
    }

    // ========================================
    // Integration test: Full workflow
    // ========================================

    #[test]
    fn test_tab_management_workflow() {
        let mut shell = create_test_shell();

        // Create 3 tabs
        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();
        let tab3 = shell.create_tab().unwrap();

        assert_eq!(shell.get_tab_count(), 3);

        // Switch between tabs
        shell.switch_to_tab(tab2).unwrap();
        assert_eq!(shell.get_active_tab(), Some(tab2));

        shell.switch_to_tab(tab3).unwrap();
        assert_eq!(shell.get_active_tab(), Some(tab3));

        // Close middle tab
        shell.close_tab(tab2).unwrap();
        assert_eq!(shell.get_tab_count(), 2);

        // Verify remaining tabs
        assert!(shell.get_tab(tab1).is_some());
        assert!(shell.get_tab(tab2).is_none());
        assert!(shell.get_tab(tab3).is_some());
    }
}
