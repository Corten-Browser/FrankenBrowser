//! Menu system for browser shell
//!
//! This module provides a complete menu system with keyboard shortcuts,
//! standard browser menus (File, Edit, View, History, Bookmarks, Help),
//! and event handling capabilities.

use crate::errors::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Key modifier for keyboard shortcuts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyModifier {
    /// Control key (Ctrl on Windows/Linux, Cmd on macOS)
    Ctrl,
    /// Alt key (Option on macOS)
    Alt,
    /// Shift key
    Shift,
    /// Meta key (Windows key on Windows, Cmd on macOS)
    Meta,
}

/// Key code for keyboard shortcuts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Letter key (A-Z)
    Letter(char),
    /// Number key (0-9)
    Number(char),
    /// Function key (F1-F12)
    Function(u8),
    /// Plus/Add key
    Plus,
    /// Minus/Subtract key
    Minus,
    /// Comma key
    Comma,
    /// Left arrow
    ArrowLeft,
    /// Right arrow
    ArrowRight,
}

/// Keyboard shortcut for menu items
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// Key modifiers (Ctrl, Alt, Shift, Meta)
    pub modifiers: Vec<KeyModifier>,
    /// Key code
    pub key: KeyCode,
    /// Display string (e.g., "Ctrl+T")
    pub display: String,
}

impl Shortcut {
    /// Create a new shortcut
    ///
    /// # Arguments
    ///
    /// * `modifiers` - List of key modifiers
    /// * `key` - Key code
    ///
    /// # Returns
    ///
    /// A new Shortcut instance with auto-generated display string
    pub fn new(modifiers: Vec<KeyModifier>, key: KeyCode) -> Self {
        let display = Self::generate_display(&modifiers, &key);
        Self {
            modifiers,
            key,
            display,
        }
    }

    /// Parse a shortcut from a string (e.g., "Ctrl+T", "Alt+Left")
    ///
    /// # Arguments
    ///
    /// * `shortcut_str` - Shortcut string to parse
    ///
    /// # Returns
    ///
    /// Parsed Shortcut or error if invalid format
    pub fn parse(shortcut_str: &str) -> Result<Self> {
        // Handle special case for + key (e.g., "Ctrl++")
        if shortcut_str.ends_with("++") {
            let modifier_part = &shortcut_str[..shortcut_str.len() - 2];
            let mut modifiers = Vec::new();

            if !modifier_part.is_empty() {
                for part in modifier_part.split('+') {
                    match part {
                        "Ctrl" => modifiers.push(KeyModifier::Ctrl),
                        "Alt" => modifiers.push(KeyModifier::Alt),
                        "Shift" => modifiers.push(KeyModifier::Shift),
                        "Meta" => modifiers.push(KeyModifier::Meta),
                        _ => {
                            return Err(Error::ConfigError(format!(
                                "Unknown modifier: {}",
                                part
                            )))
                        }
                    }
                }
            }

            return Ok(Self::new(modifiers, KeyCode::Plus));
        }

        let parts: Vec<&str> = shortcut_str.split('+').collect();
        if parts.is_empty() {
            return Err(Error::ConfigError("Empty shortcut string".to_string()));
        }

        let mut modifiers = Vec::new();
        let key_str = parts[parts.len() - 1];

        // Parse modifiers (all parts except the last one)
        for &part in &parts[..parts.len() - 1] {
            match part {
                "Ctrl" => modifiers.push(KeyModifier::Ctrl),
                "Alt" => modifiers.push(KeyModifier::Alt),
                "Shift" => modifiers.push(KeyModifier::Shift),
                "Meta" => modifiers.push(KeyModifier::Meta),
                "" => {}, // Skip empty parts
                _ => {
                    return Err(Error::ConfigError(format!(
                        "Unknown modifier: {}",
                        part
                    )))
                }
            }
        }

        // Parse key
        let key = if key_str.len() == 1 && key_str.chars().next().unwrap().is_alphabetic() {
            KeyCode::Letter(key_str.chars().next().unwrap())
        } else if key_str.len() == 1 && key_str.chars().next().unwrap().is_numeric() {
            KeyCode::Number(key_str.chars().next().unwrap())
        } else if key_str.starts_with('F') && key_str.len() >= 2 {
            let num_str = &key_str[1..];
            let num: u8 = num_str
                .parse()
                .map_err(|_| Error::ConfigError(format!("Invalid function key: {}", key_str)))?;
            KeyCode::Function(num)
        } else {
            match key_str {
                "-" => KeyCode::Minus,
                "," => KeyCode::Comma,
                "Left" => KeyCode::ArrowLeft,
                "Right" => KeyCode::ArrowRight,
                _ => {
                    return Err(Error::ConfigError(format!("Unknown key: {}", key_str)))
                }
            }
        };

        Ok(Self::new(modifiers, key))
    }

    /// Generate display string from modifiers and key
    fn generate_display(modifiers: &[KeyModifier], key: &KeyCode) -> String {
        let mut parts = Vec::new();

        for modifier in modifiers {
            parts.push(match modifier {
                KeyModifier::Ctrl => "Ctrl",
                KeyModifier::Alt => "Alt",
                KeyModifier::Shift => "Shift",
                KeyModifier::Meta => "Meta",
            });
        }

        let key_str = match key {
            KeyCode::Letter(c) => c.to_string(),
            KeyCode::Number(c) => c.to_string(),
            KeyCode::Function(n) => format!("F{}", n),
            KeyCode::Plus => "+".to_string(),
            KeyCode::Minus => "-".to_string(),
            KeyCode::Comma => ",".to_string(),
            KeyCode::ArrowLeft => "Left".to_string(),
            KeyCode::ArrowRight => "Right".to_string(),
        };
        parts.push(&key_str);

        parts.join("+")
    }
}

/// Menu action types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MenuAction {
    /// Create new tab
    NewTab,
    /// Create new window
    NewWindow,
    /// Close current tab
    CloseTab,
    /// Close current window
    CloseWindow,
    /// Quit application
    Quit,
    /// Copy selection
    Copy,
    /// Paste from clipboard
    Paste,
    /// Cut selection
    Cut,
    /// Open find dialog
    Find,
    /// Open preferences
    Preferences,
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
    /// Reset zoom to default
    ZoomReset,
    /// Toggle full screen
    FullScreen,
    /// Open developer tools
    DevTools,
    /// Navigate back
    Back,
    /// Navigate forward
    Forward,
    /// Show history
    ShowHistory,
    /// Add bookmark
    AddBookmark,
    /// Show bookmarks
    ShowBookmarks,
    /// Show about dialog
    About,
    /// Custom action with identifier
    Custom(String),
}

/// Menu item with label, shortcut, and action
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Menu item label
    pub label: String,
    /// Optional keyboard shortcut
    pub shortcut: Option<Shortcut>,
    /// Menu action to trigger
    pub action: MenuAction,
    /// Whether item is enabled
    pub enabled: bool,
    /// Whether this is a separator item
    pub is_separator: bool,
    /// Optional submenu items
    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
    /// Create a new menu item
    ///
    /// # Arguments
    ///
    /// * `label` - Menu item label
    ///
    /// # Returns
    ///
    /// A new MenuItem with default settings (enabled, no shortcut, no action)
    pub fn new(label: String) -> Self {
        Self {
            label,
            shortcut: None,
            action: MenuAction::Custom("noop".to_string()),
            enabled: true,
            is_separator: false,
            submenu: None,
        }
    }

    /// Set keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Set menu action
    pub fn with_action(mut self, action: MenuAction) -> Self {
        self.action = action;
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Create a separator menu item
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            action: MenuAction::Custom("separator".to_string()),
            enabled: false,
            is_separator: true,
            submenu: None,
        }
    }

    /// Set submenu items
    pub fn with_submenu(mut self, items: Vec<MenuItem>) -> Self {
        self.submenu = Some(items);
        self
    }
}

/// Menu with title and items
#[derive(Debug, Clone)]
pub struct Menu {
    /// Menu title
    pub title: String,
    /// Menu items
    pub items: Vec<MenuItem>,
}

impl Menu {
    /// Create a new menu
    ///
    /// # Arguments
    ///
    /// * `title` - Menu title
    ///
    /// # Returns
    ///
    /// A new Menu with no items
    pub fn new(title: String) -> Self {
        Self {
            title,
            items: Vec::new(),
        }
    }

    /// Add a menu item
    ///
    /// # Arguments
    ///
    /// * `item` - MenuItem to add
    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    /// Add a separator
    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::separator());
    }

    /// Get a menu item by label
    ///
    /// # Arguments
    ///
    /// * `label` - Label to search for
    ///
    /// # Returns
    ///
    /// Reference to MenuItem if found, None otherwise
    pub fn get_item(&self, label: &str) -> Option<&MenuItem> {
        self.items.iter().find(|item| item.label == label)
    }
}

/// Event handler function type
pub type EventHandler = Arc<dyn Fn() -> Result<()> + Send + Sync>;

/// Menu element for rendering
#[derive(Debug, Clone)]
pub struct MenuElement {
    /// Menu title
    pub title: String,
    /// Menu item label
    pub label: String,
    /// Shortcut display string
    pub shortcut: Option<String>,
    /// Whether item is enabled
    pub enabled: bool,
    /// Whether this is a separator
    pub is_separator: bool,
}

/// Menu bar containing multiple menus
pub struct MenuBar {
    /// All menus in the menu bar
    pub menus: Vec<Menu>,
    /// Event handlers for menu actions
    handlers: HashMap<MenuAction, EventHandler>,
}

impl MenuBar {
    /// Create a new menu bar with default menus
    ///
    /// # Returns
    ///
    /// A MenuBar with standard browser menus (File, Edit, View, History, Bookmarks, Help)
    pub fn new() -> Self {
        let mut menu_bar = Self {
            menus: Vec::new(),
            handlers: HashMap::new(),
        };

        menu_bar.add_menu(Self::create_file_menu());
        menu_bar.add_menu(Self::create_edit_menu());
        menu_bar.add_menu(Self::create_view_menu());
        menu_bar.add_menu(Self::create_history_menu());
        menu_bar.add_menu(Self::create_bookmarks_menu());
        menu_bar.add_menu(Self::create_help_menu());

        menu_bar
    }

    /// Add a menu to the menu bar
    ///
    /// # Arguments
    ///
    /// * `menu` - Menu to add
    pub fn add_menu(&mut self, menu: Menu) {
        self.menus.push(menu);
    }

    /// Remove a menu by title
    ///
    /// # Arguments
    ///
    /// * `title` - Title of menu to remove
    pub fn remove_menu(&mut self, title: &str) {
        self.menus.retain(|menu| menu.title != title);
    }

    /// Get a menu by title
    ///
    /// # Arguments
    ///
    /// * `title` - Title of menu to retrieve
    ///
    /// # Returns
    ///
    /// Reference to Menu if found, None otherwise
    pub fn get_menu(&self, title: &str) -> Option<&Menu> {
        self.menus.iter().find(|menu| menu.title == title)
    }

    /// Register an event handler for a menu action
    ///
    /// # Arguments
    ///
    /// * `action` - MenuAction to handle
    /// * `handler` - Event handler function
    pub fn register_handler(&mut self, action: MenuAction, handler: EventHandler) {
        self.handlers.insert(action, handler);
    }

    /// Trigger a menu action
    ///
    /// # Arguments
    ///
    /// * `action` - MenuAction to trigger
    ///
    /// # Returns
    ///
    /// Result of executing the handler, or error if no handler registered
    pub fn trigger_action(&self, action: &MenuAction) -> Result<()> {
        if let Some(handler) = self.handlers.get(action) {
            handler()
        } else {
            Err(Error::ConfigError(format!(
                "No handler registered for action: {:?}",
                action
            )))
        }
    }

    /// Find menu item by keyboard shortcut
    ///
    /// # Arguments
    ///
    /// * `shortcut` - Shortcut to search for
    ///
    /// # Returns
    ///
    /// Reference to MenuItem if found, None otherwise
    pub fn find_item_by_shortcut(&self, shortcut: &Shortcut) -> Option<&MenuItem> {
        for menu in &self.menus {
            for item in &menu.items {
                if let Some(ref item_shortcut) = item.shortcut {
                    if item_shortcut == shortcut {
                        return Some(item);
                    }
                }
            }
        }
        None
    }

    /// Render menu bar as list of menu elements for UI
    ///
    /// # Returns
    ///
    /// Vector of MenuElement for rendering
    pub fn render(&self) -> Vec<MenuElement> {
        let mut elements = Vec::new();

        for menu in &self.menus {
            for item in &menu.items {
                elements.push(MenuElement {
                    title: menu.title.clone(),
                    label: item.label.clone(),
                    shortcut: item.shortcut.as_ref().map(|s| s.display.clone()),
                    enabled: item.enabled,
                    is_separator: item.is_separator,
                });
            }
        }

        elements
    }

    /// Create File menu
    fn create_file_menu() -> Menu {
        let mut menu = Menu::new("File".to_string());

        menu.add_item(
            MenuItem::new("New Tab".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+T").unwrap())
                .with_action(MenuAction::NewTab),
        );

        menu.add_item(
            MenuItem::new("New Window".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+N").unwrap())
                .with_action(MenuAction::NewWindow),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Close Tab".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+W").unwrap())
                .with_action(MenuAction::CloseTab),
        );

        menu.add_item(
            MenuItem::new("Close Window".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+Shift+W").unwrap())
                .with_action(MenuAction::CloseWindow),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Quit".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+Q").unwrap())
                .with_action(MenuAction::Quit),
        );

        menu
    }

    /// Create Edit menu
    fn create_edit_menu() -> Menu {
        let mut menu = Menu::new("Edit".to_string());

        menu.add_item(
            MenuItem::new("Cut".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+X").unwrap())
                .with_action(MenuAction::Cut),
        );

        menu.add_item(
            MenuItem::new("Copy".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+C").unwrap())
                .with_action(MenuAction::Copy),
        );

        menu.add_item(
            MenuItem::new("Paste".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+V").unwrap())
                .with_action(MenuAction::Paste),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Find".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+F").unwrap())
                .with_action(MenuAction::Find),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Preferences".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+,").unwrap())
                .with_action(MenuAction::Preferences),
        );

        menu
    }

    /// Create View menu
    fn create_view_menu() -> Menu {
        let mut menu = Menu::new("View".to_string());

        menu.add_item(
            MenuItem::new("Zoom In".to_string())
                .with_shortcut(Shortcut::parse("Ctrl++").unwrap())
                .with_action(MenuAction::ZoomIn),
        );

        menu.add_item(
            MenuItem::new("Zoom Out".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+-").unwrap())
                .with_action(MenuAction::ZoomOut),
        );

        menu.add_item(
            MenuItem::new("Zoom Reset".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+0").unwrap())
                .with_action(MenuAction::ZoomReset),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Full Screen".to_string())
                .with_shortcut(Shortcut::parse("F11").unwrap())
                .with_action(MenuAction::FullScreen),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Developer Tools".to_string())
                .with_shortcut(Shortcut::parse("F12").unwrap())
                .with_action(MenuAction::DevTools),
        );

        menu
    }

    /// Create History menu
    fn create_history_menu() -> Menu {
        let mut menu = Menu::new("History".to_string());

        menu.add_item(
            MenuItem::new("Back".to_string())
                .with_shortcut(Shortcut::parse("Alt+Left").unwrap())
                .with_action(MenuAction::Back),
        );

        menu.add_item(
            MenuItem::new("Forward".to_string())
                .with_shortcut(Shortcut::parse("Alt+Right").unwrap())
                .with_action(MenuAction::Forward),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Show All History".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+H").unwrap())
                .with_action(MenuAction::ShowHistory),
        );

        menu
    }

    /// Create Bookmarks menu
    fn create_bookmarks_menu() -> Menu {
        let mut menu = Menu::new("Bookmarks".to_string());

        menu.add_item(
            MenuItem::new("Add Bookmark".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+D").unwrap())
                .with_action(MenuAction::AddBookmark),
        );

        menu.add_separator();

        menu.add_item(
            MenuItem::new("Show All Bookmarks".to_string())
                .with_shortcut(Shortcut::parse("Ctrl+Shift+B").unwrap())
                .with_action(MenuAction::ShowBookmarks),
        );

        menu
    }

    /// Create Help menu
    fn create_help_menu() -> Menu {
        let mut menu = Menu::new("Help".to_string());

        menu.add_item(
            MenuItem::new("About FrankenBrowser".to_string())
                .with_action(MenuAction::About),
        );

        menu
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Tests for KeyModifier and KeyCode
    // ========================================

    #[test]
    fn test_key_modifier_equality() {
        assert_eq!(KeyModifier::Ctrl, KeyModifier::Ctrl);
        assert_ne!(KeyModifier::Ctrl, KeyModifier::Alt);
    }

    #[test]
    fn test_key_code_equality() {
        assert_eq!(KeyCode::Letter('T'), KeyCode::Letter('T'));
        assert_ne!(KeyCode::Letter('T'), KeyCode::Letter('N'));
        assert_eq!(KeyCode::Function(12), KeyCode::Function(12));
        assert_ne!(KeyCode::Function(11), KeyCode::Function(12));
    }

    // ========================================
    // Tests for Shortcut
    // ========================================

    #[test]
    fn test_shortcut_new() {
        let shortcut = Shortcut::new(vec![KeyModifier::Ctrl], KeyCode::Letter('T'));
        assert_eq!(shortcut.modifiers, vec![KeyModifier::Ctrl]);
        assert_eq!(shortcut.key, KeyCode::Letter('T'));
        assert_eq!(shortcut.display, "Ctrl+T");
    }

    #[test]
    fn test_shortcut_parse_single_modifier() {
        let shortcut = Shortcut::parse("Ctrl+T").unwrap();
        assert_eq!(shortcut.modifiers, vec![KeyModifier::Ctrl]);
        assert_eq!(shortcut.key, KeyCode::Letter('T'));
        assert_eq!(shortcut.display, "Ctrl+T");
    }

    #[test]
    fn test_shortcut_parse_multiple_modifiers() {
        let shortcut = Shortcut::parse("Ctrl+Shift+W").unwrap();
        assert_eq!(
            shortcut.modifiers,
            vec![KeyModifier::Ctrl, KeyModifier::Shift]
        );
        assert_eq!(shortcut.key, KeyCode::Letter('W'));
        assert_eq!(shortcut.display, "Ctrl+Shift+W");
    }

    #[test]
    fn test_shortcut_parse_function_key() {
        let shortcut = Shortcut::parse("F12").unwrap();
        assert!(shortcut.modifiers.is_empty());
        assert_eq!(shortcut.key, KeyCode::Function(12));
        assert_eq!(shortcut.display, "F12");
    }

    #[test]
    fn test_shortcut_parse_alt_arrow() {
        let shortcut = Shortcut::parse("Alt+Left").unwrap();
        assert_eq!(shortcut.modifiers, vec![KeyModifier::Alt]);
        assert_eq!(shortcut.key, KeyCode::ArrowLeft);
        assert_eq!(shortcut.display, "Alt+Left");
    }

    #[test]
    fn test_shortcut_parse_special_keys() {
        let plus = Shortcut::parse("Ctrl++").unwrap();
        assert_eq!(plus.key, KeyCode::Plus);

        let minus = Shortcut::parse("Ctrl+-").unwrap();
        assert_eq!(minus.key, KeyCode::Minus);

        let comma = Shortcut::parse("Ctrl+,").unwrap();
        assert_eq!(comma.key, KeyCode::Comma);
    }

    #[test]
    fn test_shortcut_parse_number() {
        let shortcut = Shortcut::parse("Ctrl+0").unwrap();
        assert_eq!(shortcut.key, KeyCode::Number('0'));
        assert_eq!(shortcut.display, "Ctrl+0");
    }

    #[test]
    fn test_shortcut_parse_invalid_modifier() {
        let result = Shortcut::parse("Invalid+T");
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcut_parse_invalid_key() {
        let result = Shortcut::parse("Ctrl+Unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcut_parse_empty() {
        let result = Shortcut::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_shortcut_equality() {
        let s1 = Shortcut::parse("Ctrl+T").unwrap();
        let s2 = Shortcut::parse("Ctrl+T").unwrap();
        assert_eq!(s1, s2);

        let s3 = Shortcut::parse("Ctrl+N").unwrap();
        assert_ne!(s1, s3);
    }

    // ========================================
    // Tests for MenuAction
    // ========================================

    #[test]
    fn test_menu_action_equality() {
        assert_eq!(MenuAction::NewTab, MenuAction::NewTab);
        assert_ne!(MenuAction::NewTab, MenuAction::NewWindow);
        assert_eq!(
            MenuAction::Custom("test".to_string()),
            MenuAction::Custom("test".to_string())
        );
    }

    // ========================================
    // Tests for MenuItem
    // ========================================

    #[test]
    fn test_menu_item_new() {
        let item = MenuItem::new("New Tab".to_string());
        assert_eq!(item.label, "New Tab");
        assert!(item.shortcut.is_none());
        assert!(item.enabled);
        assert!(!item.is_separator);
        assert!(item.submenu.is_none());
    }

    #[test]
    fn test_menu_item_with_shortcut() {
        let shortcut = Shortcut::parse("Ctrl+T").unwrap();
        let item = MenuItem::new("New Tab".to_string()).with_shortcut(shortcut.clone());

        assert!(item.shortcut.is_some());
        assert_eq!(item.shortcut.unwrap(), shortcut);
    }

    #[test]
    fn test_menu_item_with_action() {
        let item = MenuItem::new("New Tab".to_string()).with_action(MenuAction::NewTab);
        assert_eq!(item.action, MenuAction::NewTab);
    }

    #[test]
    fn test_menu_item_with_enabled() {
        let item = MenuItem::new("Disabled".to_string()).with_enabled(false);
        assert!(!item.enabled);
    }

    #[test]
    fn test_menu_item_separator() {
        let item = MenuItem::separator();
        assert!(item.is_separator);
        assert!(!item.enabled);
        assert_eq!(item.label, "");
    }

    #[test]
    fn test_menu_item_with_submenu() {
        let submenu_items = vec![
            MenuItem::new("Submenu Item 1".to_string()),
            MenuItem::new("Submenu Item 2".to_string()),
        ];
        let item = MenuItem::new("Parent".to_string()).with_submenu(submenu_items.clone());

        assert!(item.submenu.is_some());
        assert_eq!(item.submenu.unwrap().len(), 2);
    }

    #[test]
    fn test_menu_item_builder_pattern() {
        let item = MenuItem::new("Complete".to_string())
            .with_shortcut(Shortcut::parse("Ctrl+T").unwrap())
            .with_action(MenuAction::NewTab)
            .with_enabled(true);

        assert_eq!(item.label, "Complete");
        assert!(item.shortcut.is_some());
        assert_eq!(item.action, MenuAction::NewTab);
        assert!(item.enabled);
    }

    // ========================================
    // Tests for Menu
    // ========================================

    #[test]
    fn test_menu_new() {
        let menu = Menu::new("File".to_string());
        assert_eq!(menu.title, "File");
        assert!(menu.items.is_empty());
    }

    #[test]
    fn test_menu_add_item() {
        let mut menu = Menu::new("File".to_string());
        let item = MenuItem::new("New Tab".to_string());

        menu.add_item(item);
        assert_eq!(menu.items.len(), 1);
        assert_eq!(menu.items[0].label, "New Tab");
    }

    #[test]
    fn test_menu_add_separator() {
        let mut menu = Menu::new("File".to_string());
        menu.add_separator();

        assert_eq!(menu.items.len(), 1);
        assert!(menu.items[0].is_separator);
    }

    #[test]
    fn test_menu_get_item() {
        let mut menu = Menu::new("File".to_string());
        menu.add_item(MenuItem::new("New Tab".to_string()));
        menu.add_item(MenuItem::new("Close Tab".to_string()));

        let item = menu.get_item("New Tab");
        assert!(item.is_some());
        assert_eq!(item.unwrap().label, "New Tab");

        let missing = menu.get_item("Missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_menu_multiple_items() {
        let mut menu = Menu::new("File".to_string());
        menu.add_item(MenuItem::new("Item 1".to_string()));
        menu.add_separator();
        menu.add_item(MenuItem::new("Item 2".to_string()));

        assert_eq!(menu.items.len(), 3);
        assert_eq!(menu.items[0].label, "Item 1");
        assert!(menu.items[1].is_separator);
        assert_eq!(menu.items[2].label, "Item 2");
    }

    // ========================================
    // Tests for MenuBar
    // ========================================

    #[test]
    fn test_menu_bar_new() {
        let menu_bar = MenuBar::new();
        // Should have 6 default menus: File, Edit, View, History, Bookmarks, Help
        assert_eq!(menu_bar.menus.len(), 6);
    }

    #[test]
    fn test_menu_bar_default() {
        let menu_bar = MenuBar::default();
        assert_eq!(menu_bar.menus.len(), 6);
    }

    #[test]
    fn test_menu_bar_add_menu() {
        let mut menu_bar = MenuBar::new();
        let custom_menu = Menu::new("Custom".to_string());

        menu_bar.add_menu(custom_menu);
        assert_eq!(menu_bar.menus.len(), 7); // 6 default + 1 custom
    }

    #[test]
    fn test_menu_bar_remove_menu() {
        let mut menu_bar = MenuBar::new();
        menu_bar.remove_menu("File");

        assert_eq!(menu_bar.menus.len(), 5);
        assert!(menu_bar.get_menu("File").is_none());
    }

    #[test]
    fn test_menu_bar_get_menu() {
        let menu_bar = MenuBar::new();

        let file_menu = menu_bar.get_menu("File");
        assert!(file_menu.is_some());
        assert_eq!(file_menu.unwrap().title, "File");

        let missing = menu_bar.get_menu("Missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_menu_bar_register_handler() {
        let mut menu_bar = MenuBar::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: EventHandler = Arc::new(move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        menu_bar.register_handler(MenuAction::NewTab, handler);
        assert_eq!(menu_bar.handlers.len(), 1);
    }

    #[test]
    fn test_menu_bar_trigger_action() {
        let mut menu_bar = MenuBar::new();
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: EventHandler = Arc::new(move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        menu_bar.register_handler(MenuAction::NewTab, handler);
        let result = menu_bar.trigger_action(&MenuAction::NewTab);

        assert!(result.is_ok());
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_menu_bar_trigger_unregistered_action() {
        let menu_bar = MenuBar::new();
        let result = menu_bar.trigger_action(&MenuAction::NewTab);

        assert!(result.is_err());
    }

    #[test]
    fn test_menu_bar_find_item_by_shortcut() {
        let menu_bar = MenuBar::new();
        let shortcut = Shortcut::parse("Ctrl+T").unwrap();

        let item = menu_bar.find_item_by_shortcut(&shortcut);
        assert!(item.is_some());
        assert_eq!(item.unwrap().label, "New Tab");
    }

    #[test]
    fn test_menu_bar_find_item_by_nonexistent_shortcut() {
        let menu_bar = MenuBar::new();
        let shortcut = Shortcut::parse("Ctrl+Z").unwrap();

        let item = menu_bar.find_item_by_shortcut(&shortcut);
        assert!(item.is_none());
    }

    #[test]
    fn test_menu_bar_render() {
        let menu_bar = MenuBar::new();
        let elements = menu_bar.render();

        // Should render all menu items from all 6 default menus
        assert!(!elements.is_empty());

        // Check first element is from File menu
        assert_eq!(elements[0].title, "File");
        assert_eq!(elements[0].label, "New Tab");
        assert!(elements[0].shortcut.is_some());
        assert!(elements[0].enabled);
    }

    // ========================================
    // Tests for Standard Menus
    // ========================================

    #[test]
    fn test_file_menu_structure() {
        let menu_bar = MenuBar::new();
        let file_menu = menu_bar.get_menu("File").unwrap();

        // Should have: New Tab, New Window, Sep, Close Tab, Close Window, Sep, Quit
        assert_eq!(file_menu.items.len(), 7);

        assert_eq!(file_menu.items[0].label, "New Tab");
        assert_eq!(file_menu.items[0].action, MenuAction::NewTab);

        assert_eq!(file_menu.items[1].label, "New Window");
        assert_eq!(file_menu.items[1].action, MenuAction::NewWindow);

        assert!(file_menu.items[2].is_separator);

        assert_eq!(file_menu.items[3].label, "Close Tab");
        assert_eq!(file_menu.items[3].action, MenuAction::CloseTab);

        assert_eq!(file_menu.items[4].label, "Close Window");
        assert_eq!(file_menu.items[4].action, MenuAction::CloseWindow);

        assert!(file_menu.items[5].is_separator);

        assert_eq!(file_menu.items[6].label, "Quit");
        assert_eq!(file_menu.items[6].action, MenuAction::Quit);
    }

    #[test]
    fn test_edit_menu_structure() {
        let menu_bar = MenuBar::new();
        let edit_menu = menu_bar.get_menu("Edit").unwrap();

        // Should have: Cut, Copy, Paste, Sep, Find, Sep, Preferences
        assert_eq!(edit_menu.items.len(), 7);

        assert_eq!(edit_menu.items[0].label, "Cut");
        assert_eq!(edit_menu.items[1].label, "Copy");
        assert_eq!(edit_menu.items[2].label, "Paste");
        assert!(edit_menu.items[3].is_separator);
        assert_eq!(edit_menu.items[4].label, "Find");
        assert!(edit_menu.items[5].is_separator);
        assert_eq!(edit_menu.items[6].label, "Preferences");
    }

    #[test]
    fn test_view_menu_structure() {
        let menu_bar = MenuBar::new();
        let view_menu = menu_bar.get_menu("View").unwrap();

        // Should have: Zoom In, Zoom Out, Zoom Reset, Sep, Full Screen, Sep, Developer Tools
        assert_eq!(view_menu.items.len(), 7);

        assert_eq!(view_menu.items[0].label, "Zoom In");
        assert_eq!(view_menu.items[1].label, "Zoom Out");
        assert_eq!(view_menu.items[2].label, "Zoom Reset");
        assert!(view_menu.items[3].is_separator);
        assert_eq!(view_menu.items[4].label, "Full Screen");
        assert!(view_menu.items[5].is_separator);
        assert_eq!(view_menu.items[6].label, "Developer Tools");
    }

    #[test]
    fn test_history_menu_structure() {
        let menu_bar = MenuBar::new();
        let history_menu = menu_bar.get_menu("History").unwrap();

        // Should have: Back, Forward, Sep, Show All History
        assert_eq!(history_menu.items.len(), 4);

        assert_eq!(history_menu.items[0].label, "Back");
        assert_eq!(history_menu.items[1].label, "Forward");
        assert!(history_menu.items[2].is_separator);
        assert_eq!(history_menu.items[3].label, "Show All History");
    }

    #[test]
    fn test_bookmarks_menu_structure() {
        let menu_bar = MenuBar::new();
        let bookmarks_menu = menu_bar.get_menu("Bookmarks").unwrap();

        // Should have: Add Bookmark, Sep, Show All Bookmarks
        assert_eq!(bookmarks_menu.items.len(), 3);

        assert_eq!(bookmarks_menu.items[0].label, "Add Bookmark");
        assert!(bookmarks_menu.items[1].is_separator);
        assert_eq!(bookmarks_menu.items[2].label, "Show All Bookmarks");
    }

    #[test]
    fn test_help_menu_structure() {
        let menu_bar = MenuBar::new();
        let help_menu = menu_bar.get_menu("Help").unwrap();

        // Should have: About FrankenBrowser
        assert_eq!(help_menu.items.len(), 1);
        assert_eq!(help_menu.items[0].label, "About FrankenBrowser");
    }

    // ========================================
    // Tests for Keyboard Shortcuts
    // ========================================

    #[test]
    fn test_all_file_menu_shortcuts() {
        let menu_bar = MenuBar::new();
        let file_menu = menu_bar.get_menu("File").unwrap();

        assert_eq!(
            file_menu.items[0].shortcut.as_ref().unwrap().display,
            "Ctrl+T"
        );
        assert_eq!(
            file_menu.items[1].shortcut.as_ref().unwrap().display,
            "Ctrl+N"
        );
        assert_eq!(
            file_menu.items[3].shortcut.as_ref().unwrap().display,
            "Ctrl+W"
        );
        assert_eq!(
            file_menu.items[4].shortcut.as_ref().unwrap().display,
            "Ctrl+Shift+W"
        );
        assert_eq!(
            file_menu.items[6].shortcut.as_ref().unwrap().display,
            "Ctrl+Q"
        );
    }

    #[test]
    fn test_all_view_menu_shortcuts() {
        let menu_bar = MenuBar::new();
        let view_menu = menu_bar.get_menu("View").unwrap();

        assert_eq!(
            view_menu.items[0].shortcut.as_ref().unwrap().display,
            "Ctrl++"
        );
        assert_eq!(
            view_menu.items[1].shortcut.as_ref().unwrap().display,
            "Ctrl+-"
        );
        assert_eq!(
            view_menu.items[2].shortcut.as_ref().unwrap().display,
            "Ctrl+0"
        );
        assert_eq!(
            view_menu.items[4].shortcut.as_ref().unwrap().display,
            "F11"
        );
        assert_eq!(
            view_menu.items[6].shortcut.as_ref().unwrap().display,
            "F12"
        );
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_complete_menu_workflow() {
        let mut menu_bar = MenuBar::new();

        // Register handler
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        let handler: EventHandler = Arc::new(move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });
        menu_bar.register_handler(MenuAction::NewTab, handler);

        // Find menu item by shortcut
        let shortcut = Shortcut::parse("Ctrl+T").unwrap();
        let item = menu_bar.find_item_by_shortcut(&shortcut);
        assert!(item.is_some());

        // Trigger action
        let result = menu_bar.trigger_action(&item.unwrap().action);
        assert!(result.is_ok());
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_custom_menu_action() {
        let custom_action = MenuAction::Custom("my_action".to_string());
        let item = MenuItem::new("Custom".to_string()).with_action(custom_action.clone());

        assert_eq!(item.action, custom_action);
    }

    #[test]
    fn test_menu_element_rendering() {
        let menu_bar = MenuBar::new();
        let elements = menu_bar.render();

        // Find New Tab element
        let new_tab = elements
            .iter()
            .find(|e| e.label == "New Tab")
            .unwrap();

        assert_eq!(new_tab.title, "File");
        assert_eq!(new_tab.shortcut, Some("Ctrl+T".to_string()));
        assert!(new_tab.enabled);
        assert!(!new_tab.is_separator);
    }

    #[test]
    fn test_separator_rendering() {
        let menu_bar = MenuBar::new();
        let elements = menu_bar.render();

        // Find a separator
        let separator = elements.iter().find(|e| e.is_separator).unwrap();

        assert_eq!(separator.label, "");
        assert!(!separator.enabled);
        assert!(separator.is_separator);
    }
}
