//! Core types for browser shell

use crate::errors::{Error, Result};
use crate::menu::{MenuAction, MenuBar};
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
    /// Menu bar with keyboard shortcuts and actions
    menu_bar: MenuBar,
    /// UI Components (headless-compatible)
    url_bar: crate::ui_components::URLBar,
    navigation_buttons: crate::ui_components::NavigationButtons,
    tab_bar: crate::ui_components::TabBar,
    status_bar: crate::ui_components::StatusBar,
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

            // Create WebView using wry 0.53 API
            // In wry 0.53+, WebViewBuilder::new() takes no parameters,
            // and .build(&window) takes the window reference
            let webview = WebViewBuilder::new()
                .with_url(&config.homepage)
                .build(&window)
                .map_err(|e| Error::Initialization(format!("Failed to build webview: {}", e)))?;

            Ok(Self {
                config,
                message_sender: sender,
                runtime,
                tabs: HashMap::new(),
                active_tab: None,
                next_tab_id: 1,
                menu_bar: MenuBar::new(),
                url_bar: crate::ui_components::URLBar::new(),
                navigation_buttons: crate::ui_components::NavigationButtons::new(),
                tab_bar: crate::ui_components::TabBar::new(),
                status_bar: crate::ui_components::StatusBar::new(),
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
                menu_bar: MenuBar::new(),
                url_bar: crate::ui_components::URLBar::new(),
                navigation_buttons: crate::ui_components::NavigationButtons::new(),
                tab_bar: crate::ui_components::TabBar::new(),
                status_bar: crate::ui_components::StatusBar::new(),
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
        self.tabs.insert(tab_id, tab.clone());

        // Set as active tab if it's the first tab
        if self.active_tab.is_none() {
            self.active_tab = Some(tab_id);
        }

        // Update UI: Add tab to tab bar
        let _ = self.tab_bar.add_tab(tab_id, tab.title.clone());

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

    /// Get a reference to the menu bar
    ///
    /// # Returns
    ///
    /// Reference to the MenuBar
    pub fn menu_bar(&self) -> &MenuBar {
        &self.menu_bar
    }

    /// Get a mutable reference to the menu bar
    ///
    /// # Returns
    ///
    /// Mutable reference to the MenuBar
    pub fn menu_bar_mut(&mut self) -> &mut MenuBar {
        &mut self.menu_bar
    }

    /// Handle a menu action
    ///
    /// # Arguments
    ///
    /// * `action` - The menu action to handle
    ///
    /// # Returns
    ///
    /// Result of handling the action
    ///
    /// # Errors
    ///
    /// Returns an error if the action handler fails or is not registered
    pub fn handle_menu_action(&mut self, action: &MenuAction) -> Result<()> {
        // First try to trigger registered handler
        if self.menu_bar.trigger_action(action).is_ok() {
            return Ok(());
        }

        // Fall back to default handling for standard actions
        match action {
            MenuAction::NewTab => {
                self.create_tab()?;
                Ok(())
            }
            MenuAction::CloseTab => {
                if let Some(tab_id) = self.active_tab {
                    self.close_tab(tab_id)?;
                }
                Ok(())
            }
            MenuAction::Quit => {
                // In headless mode, just return Ok
                // In GUI mode, this would signal the event loop to exit
                Ok(())
            }
            _ => Err(Error::ConfigError(format!(
                "No handler for action: {:?}",
                action
            ))),
        }
    }

    /// Enable or disable a menu item
    ///
    /// # Arguments
    ///
    /// * `menu_title` - Title of the menu containing the item
    /// * `item_label` - Label of the menu item
    /// * `enabled` - Whether to enable or disable the item
    ///
    /// # Returns
    ///
    /// Result of the operation
    ///
    /// # Errors
    ///
    /// Returns an error if the menu or item is not found
    pub fn set_menu_item_enabled(
        &mut self,
        menu_title: &str,
        item_label: &str,
        enabled: bool,
    ) -> Result<()> {
        // Find the menu
        let menu = self
            .menu_bar
            .menus
            .iter_mut()
            .find(|m| m.title == menu_title)
            .ok_or_else(|| Error::ConfigError(format!("Menu not found: {}", menu_title)))?;

        // Find the item
        let item = menu
            .items
            .iter_mut()
            .find(|i| i.label == item_label)
            .ok_or_else(|| Error::ConfigError(format!("Menu item not found: {}", item_label)))?;

        item.enabled = enabled;
        Ok(())
    }

    /// Update menu item states based on browser state
    ///
    /// This method updates the enabled/disabled state of menu items
    /// based on the current browser state (e.g., disable "Close Tab"
    /// when no tabs are open).
    pub fn update_menu_states(&mut self) {
        let has_active_tab = self.active_tab.is_some();

        // Update Close Tab based on active tab
        let _ = self.set_menu_item_enabled("File", "Close Tab", has_active_tab);

        // Update navigation buttons based on history (not implemented yet)
        // This is a placeholder for future implementation
        let _ = self.set_menu_item_enabled("History", "Back", false);
        let _ = self.set_menu_item_enabled("History", "Forward", false);
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
    // These tests run in headless mode only to avoid GTK threading issues

    #[test]
    #[cfg(not(feature = "gui"))]
    fn test_browser_shell_new() {
        let shell = create_test_shell();
        assert_eq!(shell.get_tab_count(), 0);
        assert_eq!(shell.get_active_tab(), None);
    }

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_create_tab() {
        let mut shell = create_test_shell();
        let tab_id = shell.create_tab().unwrap();

        assert_eq!(tab_id, 1);
        assert_eq!(shell.get_tab_count(), 1);
        assert_eq!(shell.get_active_tab(), Some(1));
    }

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_close_tab() {
        let mut shell = create_test_shell();
        let tab_id = shell.create_tab().unwrap();

        shell.close_tab(tab_id).unwrap();
        assert_eq!(shell.get_tab_count(), 0);
        assert_eq!(shell.get_active_tab(), None);
    }

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_switch_to_tab() {
        let mut shell = create_test_shell();

        let tab1 = shell.create_tab().unwrap();
        let tab2 = shell.create_tab().unwrap();

        assert_eq!(shell.get_active_tab(), Some(tab1));

        shell.switch_to_tab(tab2).unwrap();
        assert_eq!(shell.get_active_tab(), Some(tab2));
    }

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_get_tab_not_found() {
        let shell = create_test_shell();
        let tab = shell.get_tab(999);
        assert!(tab.is_none());
    }

    #[cfg(not(feature = "gui"))]
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

    #[cfg(not(feature = "gui"))]
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

    // ========================================
    // Tests for Menu Integration
    // ========================================

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_browser_shell_has_menu_bar() {
        let shell = create_test_shell();
        let menu_bar = shell.menu_bar();

        // Should have 6 default menus
        assert_eq!(menu_bar.menus.len(), 6);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_browser_shell_menu_bar_mut() {
        let mut shell = create_test_shell();
        let menu_bar = shell.menu_bar_mut();

        // Add a custom menu
        menu_bar.add_menu(crate::menu::Menu::new("Custom".to_string()));
        assert_eq!(menu_bar.menus.len(), 7);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_new_tab() {
        let mut shell = create_test_shell();
        assert_eq!(shell.get_tab_count(), 0);

        let result = shell.handle_menu_action(&MenuAction::NewTab);
        assert!(result.is_ok());
        assert_eq!(shell.get_tab_count(), 1);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_close_tab() {
        let mut shell = create_test_shell();
        shell.create_tab().unwrap();
        assert_eq!(shell.get_tab_count(), 1);

        let result = shell.handle_menu_action(&MenuAction::CloseTab);
        assert!(result.is_ok());
        assert_eq!(shell.get_tab_count(), 0);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_close_tab_no_active() {
        let mut shell = create_test_shell();
        assert_eq!(shell.get_tab_count(), 0);

        // Should not error even when no tabs
        let result = shell.handle_menu_action(&MenuAction::CloseTab);
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_quit() {
        let mut shell = create_test_shell();
        let result = shell.handle_menu_action(&MenuAction::Quit);
        assert!(result.is_ok());
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_custom_handler() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut shell = create_test_shell();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: crate::menu::EventHandler = Arc::new(move || {
            called_clone.store(true, Ordering::SeqCst);
            Ok(())
        });

        shell
            .menu_bar_mut()
            .register_handler(MenuAction::Copy, handler);

        let result = shell.handle_menu_action(&MenuAction::Copy);
        assert!(result.is_ok());
        assert!(called.load(Ordering::SeqCst));
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_handle_menu_action_unhandled() {
        let mut shell = create_test_shell();
        let result = shell.handle_menu_action(&MenuAction::Copy);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_set_menu_item_enabled() {
        let mut shell = create_test_shell();

        let result = shell.set_menu_item_enabled("File", "New Tab", false);
        assert!(result.is_ok());

        let menu_bar = shell.menu_bar();
        let file_menu = menu_bar.get_menu("File").unwrap();
        let new_tab_item = file_menu.get_item("New Tab").unwrap();
        assert!(!new_tab_item.enabled);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_set_menu_item_enabled_invalid_menu() {
        let mut shell = create_test_shell();

        let result = shell.set_menu_item_enabled("Invalid", "New Tab", false);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_set_menu_item_enabled_invalid_item() {
        let mut shell = create_test_shell();

        let result = shell.set_menu_item_enabled("File", "Invalid", false);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_update_menu_states_no_tabs() {
        let mut shell = create_test_shell();
        shell.update_menu_states();

        let menu_bar = shell.menu_bar();
        let file_menu = menu_bar.get_menu("File").unwrap();
        let close_tab_item = file_menu.get_item("Close Tab").unwrap();

        // Close Tab should be disabled when no tabs
        assert!(!close_tab_item.enabled);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_update_menu_states_with_tabs() {
        let mut shell = create_test_shell();
        shell.create_tab().unwrap();
        shell.update_menu_states();

        let menu_bar = shell.menu_bar();
        let file_menu = menu_bar.get_menu("File").unwrap();
        let close_tab_item = file_menu.get_item("Close Tab").unwrap();

        // Close Tab should be enabled when tabs exist
        assert!(close_tab_item.enabled);
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn test_menu_integration_workflow() {
        use crate::menu::Shortcut;

        let mut shell = create_test_shell();

        // Find "New Tab" by shortcut
        let shortcut = Shortcut::parse("Ctrl+T").unwrap();
        let menu_bar = shell.menu_bar();
        let item = menu_bar.find_item_by_shortcut(&shortcut);
        assert!(item.is_some());
        assert_eq!(item.unwrap().label, "New Tab");

        // Trigger the action
        let result = shell.handle_menu_action(&MenuAction::NewTab);
        assert!(result.is_ok());
        assert_eq!(shell.get_tab_count(), 1);

        // Update menu states
        shell.update_menu_states();

        // Verify Close Tab is now enabled
        let menu_bar = shell.menu_bar();
        let file_menu = menu_bar.get_menu("File").unwrap();
        let close_tab = file_menu.get_item("Close Tab").unwrap();
        assert!(close_tab.enabled);
    }
}
