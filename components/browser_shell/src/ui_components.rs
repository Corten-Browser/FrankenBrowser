//! UI components for browser shell
//!
//! This module provides UI widget implementations for:
//! - URLBar: URL input with validation and suggestions
//! - NavigationButtons: Browser navigation controls
//! - TabBar: Tab management UI
//! - StatusBar: Status information display

use crate::errors::{Error, Result};
use std::sync::Arc;
use url::Url;

/// Event handler type for UI events
pub type EventHandler = Arc<dyn Fn() + Send + Sync>;

/// Security state indicator
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityState {
    /// HTTPS connection with valid certificate
    Secure,
    /// HTTP connection or invalid certificate
    Insecure,
    /// Unknown or not applicable
    Unknown,
}

/// URL validation state
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationState {
    /// URL is valid
    Valid,
    /// URL is invalid
    Invalid,
}

/// UI element representation (headless-compatible)
#[derive(Clone)]
pub enum UiElement {
    /// Container with child elements
    Container {
        /// Child elements
        children: Vec<UiElement>,
    },
    /// Button element
    Button {
        /// Button label
        label: String,
        /// Whether button is enabled
        enabled: bool,
        /// Event handler (None in test mode)
        #[allow(dead_code)]
        handler: Option<EventHandler>,
    },
    /// Input field element
    Input {
        /// Current value
        value: String,
        /// Placeholder text
        placeholder: String,
    },
    /// Text element
    Text {
        /// Text content
        content: String,
    },
    /// Icon element
    Icon {
        /// Icon name
        name: String,
    },
    /// Progress bar element
    ProgressBar {
        /// Progress value (0.0 to 1.0)
        value: f32,
    },
}

// Manual implementation of PartialEq for UiElement
// We compare all fields except EventHandler (which can't be compared)
impl PartialEq for UiElement {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UiElement::Container { children: c1 }, UiElement::Container { children: c2 }) => {
                c1 == c2
            }
            (
                UiElement::Button {
                    label: l1,
                    enabled: e1,
                    ..
                },
                UiElement::Button {
                    label: l2,
                    enabled: e2,
                    ..
                },
            ) => l1 == l2 && e1 == e2,
            (
                UiElement::Input {
                    value: v1,
                    placeholder: p1,
                },
                UiElement::Input {
                    value: v2,
                    placeholder: p2,
                },
            ) => v1 == v2 && p1 == p2,
            (UiElement::Text { content: c1 }, UiElement::Text { content: c2 }) => c1 == c2,
            (UiElement::Icon { name: n1 }, UiElement::Icon { name: n2 }) => n1 == n2,
            (UiElement::ProgressBar { value: v1 }, UiElement::ProgressBar { value: v2 }) => {
                (v1 - v2).abs() < f32::EPSILON
            }
            _ => false,
        }
    }
}

// Manual implementation of Debug for UiElement
impl std::fmt::Debug for UiElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UiElement::Container { children } => f
                .debug_struct("Container")
                .field("children", children)
                .finish(),
            UiElement::Button {
                label,
                enabled,
                handler,
            } => f
                .debug_struct("Button")
                .field("label", label)
                .field("enabled", enabled)
                .field("handler", &handler.is_some())
                .finish(),
            UiElement::Input { value, placeholder } => f
                .debug_struct("Input")
                .field("value", value)
                .field("placeholder", placeholder)
                .finish(),
            UiElement::Text { content } => {
                f.debug_struct("Text").field("content", content).finish()
            }
            UiElement::Icon { name } => f.debug_struct("Icon").field("name", name).finish(),
            UiElement::ProgressBar { value } => f
                .debug_struct("ProgressBar")
                .field("value", value)
                .finish(),
        }
    }
}

/// URL bar widget with input validation and suggestions
#[derive(Clone)]
pub struct URLBar {
    /// Current URL text
    url: String,
    /// Placeholder text
    placeholder: String,
    /// Whether the input is focused
    focused: bool,
    /// URL validation state
    validation_state: ValidationState,
    /// URL suggestions
    suggestions: Vec<String>,
    /// onChange event handler
    on_change: Option<EventHandler>,
    /// onSubmit event handler
    on_submit: Option<EventHandler>,
    /// onFocus event handler
    on_focus: Option<EventHandler>,
    /// onBlur event handler
    on_blur: Option<EventHandler>,
}

impl URLBar {
    /// Create a new URLBar
    pub fn new() -> Self {
        Self {
            url: String::new(),
            placeholder: "Enter URL or search term".to_string(),
            focused: false,
            validation_state: ValidationState::Valid,
            suggestions: Vec::new(),
            on_change: None,
            on_submit: None,
            on_focus: None,
            on_blur: None,
        }
    }

    /// Set the URL
    pub fn set_url(&mut self, url: String) {
        self.url = url;
        self.validation_state = if self.validate_url() {
            ValidationState::Valid
        } else {
            ValidationState::Invalid
        };

        // Trigger onChange event
        if let Some(ref handler) = self.on_change {
            handler();
        }
    }

    /// Get the current URL
    pub fn get_url(&self) -> &str {
        &self.url
    }

    /// Validate the current URL
    pub fn validate_url(&self) -> bool {
        if self.url.is_empty() {
            return true; // Empty is considered valid (placeholder state)
        }

        // Try to parse as URL
        Url::parse(&self.url).is_ok()
    }

    /// Get validation state
    pub fn validation_state(&self) -> &ValidationState {
        &self.validation_state
    }

    /// Add a suggestion
    pub fn add_suggestion(&mut self, suggestion: String) {
        if !self.suggestions.contains(&suggestion) {
            self.suggestions.push(suggestion);
        }
    }

    /// Clear all suggestions
    pub fn clear_suggestions(&mut self) {
        self.suggestions.clear();
    }

    /// Get suggestions
    pub fn get_suggestions(&self) -> &[String] {
        &self.suggestions
    }

    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) {
        let was_focused = self.focused;
        self.focused = focused;

        if focused && !was_focused {
            // Trigger onFocus event
            if let Some(ref handler) = self.on_focus {
                handler();
            }
        } else if !focused && was_focused {
            // Trigger onBlur event
            if let Some(ref handler) = self.on_blur {
                handler();
            }
        }
    }

    /// Get focus state
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Set onChange handler
    pub fn on_change(&mut self, handler: EventHandler) {
        self.on_change = Some(handler);
    }

    /// Set onSubmit handler
    pub fn on_submit(&mut self, handler: EventHandler) {
        self.on_submit = Some(handler);
    }

    /// Set onFocus handler
    pub fn on_focus_handler(&mut self, handler: EventHandler) {
        self.on_focus = Some(handler);
    }

    /// Set onBlur handler
    pub fn on_blur_handler(&mut self, handler: EventHandler) {
        self.on_blur = Some(handler);
    }

    /// Trigger submit event
    pub fn submit(&self) {
        if let Some(ref handler) = self.on_submit {
            handler();
        }
    }

    /// Render the URLBar as a UI element
    pub fn render(&self) -> UiElement {
        UiElement::Container {
            children: vec![
                UiElement::Input {
                    value: self.url.clone(),
                    placeholder: self.placeholder.clone(),
                },
            ],
        }
    }
}

impl Default for URLBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation buttons widget
#[derive(Clone)]
pub struct NavigationButtons {
    /// Back button enabled state
    back_enabled: bool,
    /// Forward button enabled state
    forward_enabled: bool,
    /// Whether page is currently loading
    loading: bool,
    /// Back button handler
    on_back: Option<EventHandler>,
    /// Forward button handler
    on_forward: Option<EventHandler>,
    /// Reload button handler
    on_reload: Option<EventHandler>,
    /// Home button handler
    on_home: Option<EventHandler>,
    /// Stop button handler
    on_stop: Option<EventHandler>,
}

impl NavigationButtons {
    /// Create new navigation buttons
    pub fn new() -> Self {
        Self {
            back_enabled: false,
            forward_enabled: false,
            loading: false,
            on_back: None,
            on_forward: None,
            on_reload: None,
            on_home: None,
            on_stop: None,
        }
    }

    /// Set back button enabled state
    pub fn set_back_enabled(&mut self, enabled: bool) {
        self.back_enabled = enabled;
    }

    /// Get back button enabled state
    pub fn is_back_enabled(&self) -> bool {
        self.back_enabled
    }

    /// Set forward button enabled state
    pub fn set_forward_enabled(&mut self, enabled: bool) {
        self.forward_enabled = enabled;
    }

    /// Get forward button enabled state
    pub fn is_forward_enabled(&self) -> bool {
        self.forward_enabled
    }

    /// Set loading state (shows/hides stop button)
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Get loading state
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Set back button click handler
    pub fn on_back_click(&mut self, handler: EventHandler) {
        self.on_back = Some(handler);
    }

    /// Set forward button click handler
    pub fn on_forward_click(&mut self, handler: EventHandler) {
        self.on_forward = Some(handler);
    }

    /// Set reload button click handler
    pub fn on_reload_click(&mut self, handler: EventHandler) {
        self.on_reload = Some(handler);
    }

    /// Set home button click handler
    pub fn on_home_click(&mut self, handler: EventHandler) {
        self.on_home = Some(handler);
    }

    /// Set stop button click handler
    pub fn on_stop_click(&mut self, handler: EventHandler) {
        self.on_stop = Some(handler);
    }

    /// Trigger back button click
    pub fn click_back(&self) -> Result<()> {
        if !self.back_enabled {
            return Err(Error::WindowError("Back button is disabled".to_string()));
        }
        if let Some(ref handler) = self.on_back {
            handler();
        }
        Ok(())
    }

    /// Trigger forward button click
    pub fn click_forward(&self) -> Result<()> {
        if !self.forward_enabled {
            return Err(Error::WindowError("Forward button is disabled".to_string()));
        }
        if let Some(ref handler) = self.on_forward {
            handler();
        }
        Ok(())
    }

    /// Trigger reload button click
    pub fn click_reload(&self) {
        if let Some(ref handler) = self.on_reload {
            handler();
        }
    }

    /// Trigger home button click
    pub fn click_home(&self) {
        if let Some(ref handler) = self.on_home {
            handler();
        }
    }

    /// Trigger stop button click
    pub fn click_stop(&self) {
        if let Some(ref handler) = self.on_stop {
            handler();
        }
    }

    /// Render navigation buttons as UI element
    pub fn render(&self) -> UiElement {
        let mut buttons = vec![
            UiElement::Button {
                label: "Back".to_string(),
                enabled: self.back_enabled,
                handler: self.on_back.clone(),
            },
            UiElement::Button {
                label: "Forward".to_string(),
                enabled: self.forward_enabled,
                handler: self.on_forward.clone(),
            },
        ];

        if self.loading {
            buttons.push(UiElement::Button {
                label: "Stop".to_string(),
                enabled: true,
                handler: self.on_stop.clone(),
            });
        } else {
            buttons.push(UiElement::Button {
                label: "Reload".to_string(),
                enabled: true,
                handler: self.on_reload.clone(),
            });
        }

        buttons.push(UiElement::Button {
            label: "Home".to_string(),
            enabled: true,
            handler: self.on_home.clone(),
        });

        UiElement::Container { children: buttons }
    }
}

impl Default for NavigationButtons {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual tab widget
#[derive(Debug, Clone, PartialEq)]
pub struct TabWidget {
    /// Tab ID
    pub id: u32,
    /// Tab title
    pub title: String,
    /// Favicon URL
    pub favicon: Option<String>,
    /// Whether tab is loading
    pub loading: bool,
}

impl TabWidget {
    /// Create a new tab widget
    pub fn new(id: u32, title: String) -> Self {
        Self {
            id,
            title,
            favicon: None,
            loading: false,
        }
    }

    /// Set favicon
    pub fn set_favicon(&mut self, favicon: Option<String>) {
        self.favicon = favicon;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }
}

/// Tab bar widget for managing multiple tabs
#[derive(Clone)]
pub struct TabBar {
    /// List of tabs
    tabs: Vec<TabWidget>,
    /// Active tab index
    active_tab_index: Option<usize>,
    /// Maximum number of tabs
    max_tabs: usize,
    /// Tab click handler
    on_tab_click: Option<EventHandler>,
    /// Tab close handler
    on_tab_close: Option<EventHandler>,
    /// New tab handler
    on_new_tab: Option<EventHandler>,
}

impl TabBar {
    /// Create a new TabBar with default max tabs (20)
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: None,
            max_tabs: 20,
            on_tab_click: None,
            on_tab_close: None,
            on_new_tab: None,
        }
    }

    /// Create a new TabBar with custom max tabs
    pub fn with_max_tabs(max_tabs: usize) -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_index: None,
            max_tabs,
            on_tab_click: None,
            on_tab_close: None,
            on_new_tab: None,
        }
    }

    /// Add a new tab
    pub fn add_tab(&mut self, id: u32, title: String) -> Result<()> {
        if self.tabs.len() >= self.max_tabs {
            return Err(Error::WindowError(format!(
                "Maximum tab limit ({}) reached",
                self.max_tabs
            )));
        }

        let tab = TabWidget::new(id, title);
        self.tabs.push(tab);

        // Set as active if it's the first tab
        if self.active_tab_index.is_none() {
            self.active_tab_index = Some(0);
        }

        Ok(())
    }

    /// Remove a tab by ID
    pub fn remove_tab(&mut self, id: u32) -> Result<()> {
        let position = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| Error::TabNotFound(id))?;

        self.tabs.remove(position);

        // Update active tab index
        if let Some(active_idx) = self.active_tab_index {
            if active_idx == position {
                // Removed the active tab
                if self.tabs.is_empty() {
                    self.active_tab_index = None;
                } else if position >= self.tabs.len() {
                    self.active_tab_index = Some(self.tabs.len() - 1);
                }
            } else if active_idx > position {
                // Active tab shifted left
                self.active_tab_index = Some(active_idx - 1);
            }
        }

        Ok(())
    }

    /// Set active tab by ID
    pub fn set_active_tab(&mut self, id: u32) -> Result<()> {
        let position = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| Error::TabNotFound(id))?;

        self.active_tab_index = Some(position);
        Ok(())
    }

    /// Get active tab ID
    pub fn get_active_tab_id(&self) -> Option<u32> {
        self.active_tab_index
            .and_then(|idx| self.tabs.get(idx))
            .map(|tab| tab.id)
    }

    /// Update tab title
    pub fn update_tab_title(&mut self, id: u32, title: String) -> Result<()> {
        let tab = self
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::TabNotFound(id))?;

        tab.title = title;
        Ok(())
    }

    /// Set tab loading state
    pub fn set_tab_loading(&mut self, id: u32, loading: bool) -> Result<()> {
        let tab = self
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| Error::TabNotFound(id))?;

        tab.loading = loading;
        Ok(())
    }

    /// Get tab count
    pub fn get_tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get tab by ID
    pub fn get_tab(&self, id: u32) -> Option<&TabWidget> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Set tab click handler
    pub fn on_tab_click(&mut self, handler: EventHandler) {
        self.on_tab_click = Some(handler);
    }

    /// Set tab close handler
    pub fn on_tab_close(&mut self, handler: EventHandler) {
        self.on_tab_close = Some(handler);
    }

    /// Set new tab handler
    pub fn on_new_tab(&mut self, handler: EventHandler) {
        self.on_new_tab = Some(handler);
    }

    /// Trigger new tab event
    pub fn click_new_tab(&self) {
        if let Some(ref handler) = self.on_new_tab {
            handler();
        }
    }

    /// Render tab bar as UI element
    pub fn render(&self) -> UiElement {
        let mut children: Vec<UiElement> = self
            .tabs
            .iter()
            .map(|tab| {
                UiElement::Container {
                    children: vec![
                        UiElement::Text {
                            content: tab.title.clone(),
                        },
                        if tab.loading {
                            UiElement::Icon {
                                name: "loading".to_string(),
                            }
                        } else {
                            UiElement::Icon {
                                name: "close".to_string(),
                            }
                        },
                    ],
                }
            })
            .collect();

        // Add new tab button
        children.push(UiElement::Button {
            label: "+".to_string(),
            enabled: self.tabs.len() < self.max_tabs,
            handler: self.on_new_tab.clone(),
        });

        UiElement::Container { children }
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Status bar widget
#[derive(Clone)]
pub struct StatusBar {
    /// Status text
    status: String,
    /// Loading indicator
    loading: bool,
    /// Security state
    security_state: SecurityState,
    /// Progress (0.0 to 1.0)
    progress: Option<f32>,
}

impl StatusBar {
    /// Create a new StatusBar
    pub fn new() -> Self {
        Self {
            status: String::new(),
            loading: false,
            security_state: SecurityState::Unknown,
            progress: None,
        }
    }

    /// Set status text
    pub fn set_status(&mut self, text: String) {
        self.status = text;
    }

    /// Get status text
    pub fn get_status(&self) -> &str {
        &self.status
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if !loading {
            self.progress = None;
        }
    }

    /// Get loading state
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Set security state
    pub fn set_security_state(&mut self, state: SecurityState) {
        self.security_state = state;
    }

    /// Get security state
    pub fn get_security_state(&self) -> &SecurityState {
        &self.security_state
    }

    /// Set progress (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: Option<f32>) {
        self.progress = progress.map(|p| p.clamp(0.0, 1.0));
    }

    /// Get progress
    pub fn get_progress(&self) -> Option<f32> {
        self.progress
    }

    /// Render status bar as UI element
    pub fn render(&self) -> UiElement {
        let mut children = vec![
            UiElement::Icon {
                name: match self.security_state {
                    SecurityState::Secure => "lock",
                    SecurityState::Insecure => "unlock",
                    SecurityState::Unknown => "info",
                }
                .to_string(),
            },
            UiElement::Text {
                content: self.status.clone(),
            },
        ];

        if self.loading {
            children.push(UiElement::Icon {
                name: "loading".to_string(),
            });
        }

        if let Some(progress) = self.progress {
            children.push(UiElement::ProgressBar { value: progress });
        }

        UiElement::Container { children }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ========================================
    // URLBar Tests
    // ========================================

    #[test]
    fn test_urlbar_new() {
        let urlbar = URLBar::new();
        assert_eq!(urlbar.get_url(), "");
        assert_eq!(urlbar.is_focused(), false);
        assert_eq!(*urlbar.validation_state(), ValidationState::Valid);
        assert_eq!(urlbar.get_suggestions().len(), 0);
    }

    #[test]
    fn test_urlbar_set_url_valid() {
        let mut urlbar = URLBar::new();
        urlbar.set_url("https://www.example.com".to_string());
        assert_eq!(urlbar.get_url(), "https://www.example.com");
        assert_eq!(*urlbar.validation_state(), ValidationState::Valid);
    }

    #[test]
    fn test_urlbar_set_url_invalid() {
        let mut urlbar = URLBar::new();
        urlbar.set_url("not a valid url".to_string());
        assert_eq!(urlbar.get_url(), "not a valid url");
        assert_eq!(*urlbar.validation_state(), ValidationState::Invalid);
    }

    #[test]
    fn test_urlbar_validate_url() {
        let mut urlbar = URLBar::new();

        // Empty is valid
        assert!(urlbar.validate_url());

        // Valid URLs
        urlbar.set_url("https://www.example.com".to_string());
        assert!(urlbar.validate_url());

        urlbar.set_url("http://localhost:8080/path".to_string());
        assert!(urlbar.validate_url());

        // Invalid URLs
        urlbar.set_url("not a url".to_string());
        assert!(!urlbar.validate_url());
    }

    #[test]
    fn test_urlbar_suggestions() {
        let mut urlbar = URLBar::new();

        urlbar.add_suggestion("https://www.example.com".to_string());
        urlbar.add_suggestion("https://www.test.com".to_string());
        assert_eq!(urlbar.get_suggestions().len(), 2);

        // Duplicate suggestions should not be added
        urlbar.add_suggestion("https://www.example.com".to_string());
        assert_eq!(urlbar.get_suggestions().len(), 2);

        urlbar.clear_suggestions();
        assert_eq!(urlbar.get_suggestions().len(), 0);
    }

    #[test]
    fn test_urlbar_focus() {
        let mut urlbar = URLBar::new();

        assert!(!urlbar.is_focused());

        urlbar.set_focus(true);
        assert!(urlbar.is_focused());

        urlbar.set_focus(false);
        assert!(!urlbar.is_focused());
    }

    #[test]
    fn test_urlbar_event_handlers() {
        let mut urlbar = URLBar::new();

        let changed = Arc::new(AtomicBool::new(false));
        let changed_clone = changed.clone();
        urlbar.on_change(Arc::new(move || {
            changed_clone.store(true, Ordering::SeqCst);
        }));

        urlbar.set_url("https://www.example.com".to_string());
        assert!(changed.load(Ordering::SeqCst));

        let submitted = Arc::new(AtomicBool::new(false));
        let submitted_clone = submitted.clone();
        urlbar.on_submit(Arc::new(move || {
            submitted_clone.store(true, Ordering::SeqCst);
        }));

        urlbar.submit();
        assert!(submitted.load(Ordering::SeqCst));
    }

    #[test]
    fn test_urlbar_focus_handlers() {
        let mut urlbar = URLBar::new();

        let focused = Arc::new(AtomicBool::new(false));
        let focused_clone = focused.clone();
        urlbar.on_focus_handler(Arc::new(move || {
            focused_clone.store(true, Ordering::SeqCst);
        }));

        let blurred = Arc::new(AtomicBool::new(false));
        let blurred_clone = blurred.clone();
        urlbar.on_blur_handler(Arc::new(move || {
            blurred_clone.store(true, Ordering::SeqCst);
        }));

        urlbar.set_focus(true);
        assert!(focused.load(Ordering::SeqCst));

        urlbar.set_focus(false);
        assert!(blurred.load(Ordering::SeqCst));
    }

    #[test]
    fn test_urlbar_render() {
        let urlbar = URLBar::new();
        let rendered = urlbar.render();

        match rendered {
            UiElement::Container { children } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    UiElement::Input { value, placeholder } => {
                        assert_eq!(value, "");
                        assert_eq!(placeholder, "Enter URL or search term");
                    }
                    _ => panic!("Expected Input element"),
                }
            }
            _ => panic!("Expected Container element"),
        }
    }

    // ========================================
    // NavigationButtons Tests
    // ========================================

    #[test]
    fn test_navigation_buttons_new() {
        let nav = NavigationButtons::new();
        assert!(!nav.is_back_enabled());
        assert!(!nav.is_forward_enabled());
        assert!(!nav.is_loading());
    }

    #[test]
    fn test_navigation_buttons_enable_disable() {
        let mut nav = NavigationButtons::new();

        nav.set_back_enabled(true);
        assert!(nav.is_back_enabled());

        nav.set_forward_enabled(true);
        assert!(nav.is_forward_enabled());

        nav.set_back_enabled(false);
        assert!(!nav.is_back_enabled());
    }

    #[test]
    fn test_navigation_buttons_loading_state() {
        let mut nav = NavigationButtons::new();

        nav.set_loading(true);
        assert!(nav.is_loading());

        nav.set_loading(false);
        assert!(!nav.is_loading());
    }

    #[test]
    fn test_navigation_buttons_click_handlers() {
        let mut nav = NavigationButtons::new();

        let back_clicked = Arc::new(AtomicBool::new(false));
        let back_clone = back_clicked.clone();
        nav.on_back_click(Arc::new(move || {
            back_clone.store(true, Ordering::SeqCst);
        }));

        let forward_clicked = Arc::new(AtomicBool::new(false));
        let forward_clone = forward_clicked.clone();
        nav.on_forward_click(Arc::new(move || {
            forward_clone.store(true, Ordering::SeqCst);
        }));

        nav.set_back_enabled(true);
        nav.set_forward_enabled(true);

        nav.click_back().unwrap();
        assert!(back_clicked.load(Ordering::SeqCst));

        nav.click_forward().unwrap();
        assert!(forward_clicked.load(Ordering::SeqCst));
    }

    #[test]
    fn test_navigation_buttons_disabled_click() {
        let nav = NavigationButtons::new();
        // Back and forward are disabled by default
        assert!(nav.click_back().is_err());
        assert!(nav.click_forward().is_err());
    }

    #[test]
    fn test_navigation_buttons_reload_home_stop() {
        let mut nav = NavigationButtons::new();

        let reload_clicked = Arc::new(AtomicBool::new(false));
        let reload_clone = reload_clicked.clone();
        nav.on_reload_click(Arc::new(move || {
            reload_clone.store(true, Ordering::SeqCst);
        }));

        let home_clicked = Arc::new(AtomicBool::new(false));
        let home_clone = home_clicked.clone();
        nav.on_home_click(Arc::new(move || {
            home_clone.store(true, Ordering::SeqCst);
        }));

        let stop_clicked = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_clicked.clone();
        nav.on_stop_click(Arc::new(move || {
            stop_clone.store(true, Ordering::SeqCst);
        }));

        nav.click_reload();
        assert!(reload_clicked.load(Ordering::SeqCst));

        nav.click_home();
        assert!(home_clicked.load(Ordering::SeqCst));

        nav.click_stop();
        assert!(stop_clicked.load(Ordering::SeqCst));
    }

    #[test]
    fn test_navigation_buttons_render() {
        let mut nav = NavigationButtons::new();
        nav.set_back_enabled(true);

        let rendered = nav.render();
        match rendered {
            UiElement::Container { children } => {
                assert_eq!(children.len(), 4); // Back, Forward, Reload, Home
            }
            _ => panic!("Expected Container element"),
        }
    }

    #[test]
    fn test_navigation_buttons_render_loading() {
        let mut nav = NavigationButtons::new();
        nav.set_loading(true);

        let rendered = nav.render();
        match rendered {
            UiElement::Container { children } => {
                assert_eq!(children.len(), 4); // Back, Forward, Stop, Home
                // Third button should be Stop when loading
                match &children[2] {
                    UiElement::Button { label, .. } => {
                        assert_eq!(label, "Stop");
                    }
                    _ => panic!("Expected Button element"),
                }
            }
            _ => panic!("Expected Container element"),
        }
    }

    // ========================================
    // TabBar Tests
    // ========================================

    #[test]
    fn test_tabbar_new() {
        let tabbar = TabBar::new();
        assert_eq!(tabbar.get_tab_count(), 0);
        assert_eq!(tabbar.get_active_tab_id(), None);
    }

    #[test]
    fn test_tabbar_add_tab() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        assert_eq!(tabbar.get_tab_count(), 1);
        assert_eq!(tabbar.get_active_tab_id(), Some(1));
    }

    #[test]
    fn test_tabbar_add_multiple_tabs() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        tabbar.add_tab(2, "Tab 2".to_string()).unwrap();
        tabbar.add_tab(3, "Tab 3".to_string()).unwrap();

        assert_eq!(tabbar.get_tab_count(), 3);
        assert_eq!(tabbar.get_active_tab_id(), Some(1)); // First tab is active
    }

    #[test]
    fn test_tabbar_max_tabs() {
        let mut tabbar = TabBar::with_max_tabs(2);
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        tabbar.add_tab(2, "Tab 2".to_string()).unwrap();

        let result = tabbar.add_tab(3, "Tab 3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_tabbar_remove_tab() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        tabbar.add_tab(2, "Tab 2".to_string()).unwrap();

        tabbar.remove_tab(1).unwrap();
        assert_eq!(tabbar.get_tab_count(), 1);
        assert_eq!(tabbar.get_active_tab_id(), Some(2));
    }

    #[test]
    fn test_tabbar_remove_tab_not_found() {
        let mut tabbar = TabBar::new();
        let result = tabbar.remove_tab(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_tabbar_set_active_tab() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        tabbar.add_tab(2, "Tab 2".to_string()).unwrap();

        tabbar.set_active_tab(2).unwrap();
        assert_eq!(tabbar.get_active_tab_id(), Some(2));
    }

    #[test]
    fn test_tabbar_set_active_tab_not_found() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();

        let result = tabbar.set_active_tab(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_tabbar_update_tab_title() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();

        tabbar
            .update_tab_title(1, "Updated Title".to_string())
            .unwrap();

        let tab = tabbar.get_tab(1).unwrap();
        assert_eq!(tab.title, "Updated Title");
    }

    #[test]
    fn test_tabbar_set_tab_loading() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();

        tabbar.set_tab_loading(1, true).unwrap();
        let tab = tabbar.get_tab(1).unwrap();
        assert!(tab.loading);

        tabbar.set_tab_loading(1, false).unwrap();
        let tab = tabbar.get_tab(1).unwrap();
        assert!(!tab.loading);
    }

    #[test]
    fn test_tabbar_event_handlers() {
        let mut tabbar = TabBar::new();

        let new_tab_clicked = Arc::new(AtomicBool::new(false));
        let new_tab_clone = new_tab_clicked.clone();
        tabbar.on_new_tab(Arc::new(move || {
            new_tab_clone.store(true, Ordering::SeqCst);
        }));

        tabbar.click_new_tab();
        assert!(new_tab_clicked.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tabbar_render() {
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Tab 1".to_string()).unwrap();
        tabbar.add_tab(2, "Tab 2".to_string()).unwrap();

        let rendered = tabbar.render();
        match rendered {
            UiElement::Container { children } => {
                assert_eq!(children.len(), 3); // 2 tabs + new tab button
            }
            _ => panic!("Expected Container element"),
        }
    }

    // ========================================
    // StatusBar Tests
    // ========================================

    #[test]
    fn test_statusbar_new() {
        let statusbar = StatusBar::new();
        assert_eq!(statusbar.get_status(), "");
        assert!(!statusbar.is_loading());
        assert_eq!(*statusbar.get_security_state(), SecurityState::Unknown);
        assert_eq!(statusbar.get_progress(), None);
    }

    #[test]
    fn test_statusbar_set_status() {
        let mut statusbar = StatusBar::new();
        statusbar.set_status("Loading...".to_string());
        assert_eq!(statusbar.get_status(), "Loading...");
    }

    #[test]
    fn test_statusbar_set_loading() {
        let mut statusbar = StatusBar::new();
        statusbar.set_loading(true);
        assert!(statusbar.is_loading());

        statusbar.set_loading(false);
        assert!(!statusbar.is_loading());
    }

    #[test]
    fn test_statusbar_set_security_state() {
        let mut statusbar = StatusBar::new();

        statusbar.set_security_state(SecurityState::Secure);
        assert_eq!(*statusbar.get_security_state(), SecurityState::Secure);

        statusbar.set_security_state(SecurityState::Insecure);
        assert_eq!(*statusbar.get_security_state(), SecurityState::Insecure);
    }

    #[test]
    fn test_statusbar_set_progress() {
        let mut statusbar = StatusBar::new();

        statusbar.set_progress(Some(0.5));
        assert_eq!(statusbar.get_progress(), Some(0.5));

        // Test clamping
        statusbar.set_progress(Some(1.5));
        assert_eq!(statusbar.get_progress(), Some(1.0));

        statusbar.set_progress(Some(-0.5));
        assert_eq!(statusbar.get_progress(), Some(0.0));

        statusbar.set_progress(None);
        assert_eq!(statusbar.get_progress(), None);
    }

    #[test]
    fn test_statusbar_loading_clears_progress() {
        let mut statusbar = StatusBar::new();
        statusbar.set_progress(Some(0.5));
        statusbar.set_loading(false);
        assert_eq!(statusbar.get_progress(), None);
    }

    #[test]
    fn test_statusbar_render() {
        let mut statusbar = StatusBar::new();
        statusbar.set_status("Ready".to_string());
        statusbar.set_security_state(SecurityState::Secure);

        let rendered = statusbar.render();
        match rendered {
            UiElement::Container { children } => {
                assert!(children.len() >= 2); // Icon + Text at minimum
            }
            _ => panic!("Expected Container element"),
        }
    }

    #[test]
    fn test_statusbar_render_with_loading() {
        let mut statusbar = StatusBar::new();
        statusbar.set_loading(true);
        statusbar.set_progress(Some(0.7));

        let rendered = statusbar.render();
        match rendered {
            UiElement::Container { children } => {
                assert!(children.len() >= 3); // Icon + Text + Loading + Progress
            }
            _ => panic!("Expected Container element"),
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_full_ui_workflow() {
        // URLBar
        let mut urlbar = URLBar::new();
        urlbar.set_url("https://www.example.com".to_string());
        assert!(urlbar.validate_url());

        // NavigationButtons
        let mut nav = NavigationButtons::new();
        nav.set_back_enabled(true);
        nav.set_loading(true);
        assert!(nav.is_loading());

        // TabBar
        let mut tabbar = TabBar::new();
        tabbar.add_tab(1, "Example".to_string()).unwrap();
        tabbar.set_tab_loading(1, true).unwrap();
        assert_eq!(tabbar.get_tab_count(), 1);

        // StatusBar
        let mut statusbar = StatusBar::new();
        statusbar.set_status("Loading https://www.example.com".to_string());
        statusbar.set_loading(true);
        statusbar.set_progress(Some(0.5));
        statusbar.set_security_state(SecurityState::Secure);

        // Verify all components working together
        assert_eq!(urlbar.get_url(), "https://www.example.com");
        assert!(nav.is_loading());
        assert_eq!(tabbar.get_active_tab_id(), Some(1));
        assert!(statusbar.is_loading());
    }
}
