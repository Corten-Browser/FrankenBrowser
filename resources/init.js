/**
 * FrankenBrowser WebView Initialization Script
 *
 * This script is injected into every page loaded by the browser.
 * It provides:
 * - IPC communication channel between WebView and Rust
 * - Console redirection for debugging
 * - Error handling and reporting
 * - Browser API polyfills
 */

(function() {
    'use strict';

    // Prevent re-initialization
    if (window.__frankenbrowser_initialized) {
        return;
    }
    window.__frankenbrowser_initialized = true;

    /**
     * IPC Message Handler
     * Provides bidirectional communication between JavaScript and Rust
     */
    window.ipc = {
        /**
         * Send a message to Rust backend
         * @param {string} channel - Message channel/type
         * @param {*} data - Message payload (must be JSON-serializable)
         */
        send: function(channel, data) {
            try {
                const message = {
                    channel: channel,
                    data: data,
                    timestamp: Date.now()
                };

                // Use platform-specific IPC mechanism
                if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc) {
                    // macOS/iOS WebKit
                    window.webkit.messageHandlers.ipc.postMessage(message);
                } else if (window.chrome && window.chrome.webview) {
                    // Windows WebView2
                    window.chrome.webview.postMessage(message);
                } else if (window.external && window.external.invoke) {
                    // Legacy Windows/Linux
                    window.external.invoke(JSON.stringify(message));
                } else {
                    console.error('FrankenBrowser: No IPC mechanism available');
                }
            } catch (error) {
                console.error('FrankenBrowser IPC send error:', error);
            }
        },

        /**
         * Register a callback for messages from Rust
         * @param {string} channel - Message channel to listen on
         * @param {function} callback - Callback function(data)
         */
        on: function(channel, callback) {
            if (!window.__ipc_callbacks) {
                window.__ipc_callbacks = {};
            }

            if (!window.__ipc_callbacks[channel]) {
                window.__ipc_callbacks[channel] = [];
            }

            window.__ipc_callbacks[channel].push(callback);
        },

        /**
         * Internal: Dispatch incoming message to registered callbacks
         * @private
         */
        _dispatch: function(channel, data) {
            if (!window.__ipc_callbacks || !window.__ipc_callbacks[channel]) {
                return;
            }

            window.__ipc_callbacks[channel].forEach(function(callback) {
                try {
                    callback(data);
                } catch (error) {
                    console.error('FrankenBrowser callback error:', error);
                }
            });
        }
    };

    /**
     * Console Redirection
     * Redirect console messages to Rust for debugging
     */
    (function() {
        const originalConsole = {
            log: console.log,
            warn: console.warn,
            error: console.error,
            info: console.info,
            debug: console.debug
        };

        function createConsoleOverride(level, original) {
            return function(...args) {
                // Call original console method
                original.apply(console, args);

                // Send to Rust backend
                try {
                    const message = args.map(function(arg) {
                        if (typeof arg === 'object') {
                            try {
                                return JSON.stringify(arg);
                            } catch (e) {
                                return String(arg);
                            }
                        }
                        return String(arg);
                    }).join(' ');

                    window.ipc.send('console', {
                        level: level,
                        message: message,
                        timestamp: Date.now(),
                        url: window.location.href
                    });
                } catch (error) {
                    // Silently fail to avoid infinite loops
                }
            };
        }

        console.log = createConsoleOverride('log', originalConsole.log);
        console.warn = createConsoleOverride('warn', originalConsole.warn);
        console.error = createConsoleOverride('error', originalConsole.error);
        console.info = createConsoleOverride('info', originalConsole.info);
        console.debug = createConsoleOverride('debug', originalConsole.debug);
    })();

    /**
     * Global Error Handler
     * Catch and report uncaught errors
     */
    window.addEventListener('error', function(event) {
        window.ipc.send('error', {
            type: 'error',
            message: event.message,
            filename: event.filename,
            lineno: event.lineno,
            colno: event.colno,
            stack: event.error ? event.error.stack : null,
            url: window.location.href
        });
    }, true);

    /**
     * Promise Rejection Handler
     * Catch and report unhandled promise rejections
     */
    window.addEventListener('unhandledrejection', function(event) {
        window.ipc.send('error', {
            type: 'unhandledRejection',
            reason: event.reason ? String(event.reason) : 'Unknown',
            promise: String(event.promise),
            url: window.location.href
        });
    });

    /**
     * DOM Ready Event Listener
     * Notify Rust when DOM is ready
     */
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', function() {
            window.ipc.send('dom-ready', {
                url: window.location.href,
                title: document.title,
                readyState: document.readyState
            });
        });
    } else {
        // DOM already loaded
        window.ipc.send('dom-ready', {
            url: window.location.href,
            title: document.title,
            readyState: document.readyState
        });
    }

    /**
     * Page Load Event
     * Notify Rust when page fully loads
     */
    window.addEventListener('load', function() {
        window.ipc.send('page-load', {
            url: window.location.href,
            title: document.title,
            loadTime: performance.timing ?
                performance.timing.loadEventEnd - performance.timing.navigationStart :
                null
        });
    });

    /**
     * Navigation Events
     * Track page navigation
     */
    window.addEventListener('beforeunload', function() {
        window.ipc.send('before-unload', {
            url: window.location.href
        });
    });

    /**
     * Browser API Extensions
     * Provide additional browser functionality
     */
    window.frankenbrowser = {
        /**
         * Get browser version information
         */
        getVersion: function() {
            return {
                name: 'FrankenBrowser',
                version: '0.1.0',
                userAgent: navigator.userAgent
            };
        },

        /**
         * Request navigation to URL
         * @param {string} url - URL to navigate to
         */
        navigate: function(url) {
            window.ipc.send('navigate', { url: url });
        },

        /**
         * Go back in history
         */
        back: function() {
            window.ipc.send('navigate-back', {});
        },

        /**
         * Go forward in history
         */
        forward: function() {
            window.ipc.send('navigate-forward', {});
        },

        /**
         * Reload current page
         */
        reload: function() {
            window.ipc.send('reload', {});
        },

        /**
         * Open developer tools (if available)
         */
        openDevTools: function() {
            window.ipc.send('open-devtools', {});
        },

        /**
         * Get current page metadata
         */
        getPageInfo: function() {
            return {
                url: window.location.href,
                title: document.title,
                readyState: document.readyState,
                referrer: document.referrer,
                domain: document.domain,
                cookies: document.cookie
            };
        }
    };

    /**
     * Initialization complete notification
     */
    window.ipc.send('init-complete', {
        url: window.location.href,
        userAgent: navigator.userAgent,
        timestamp: Date.now()
    });

    console.log('FrankenBrowser initialization complete');
})();
