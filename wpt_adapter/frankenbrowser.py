"""
FrankenBrowser product adapter for Web Platform Tests (WPT)

This adapter tells WPT how to run FrankenBrowser for testing.
"""

import os
import subprocess
import time
from typing import Optional, Dict, Any

class FrankenBrowserProduct:
    """Product adapter for FrankenBrowser"""

    name = "frankenbrowser"
    vendor_prefix = "fb"

    def __init__(self, **kwargs):
        """Initialize the product adapter"""
        self.binary = kwargs.get("binary")
        self.webdriver_binary = kwargs.get("webdriver_binary")
        self.webdriver_args = kwargs.get("webdriver_args", [])
        self.webdriver_port = kwargs.get("webdriver_port", 4444)
        self.process = None

        # Find binaries if not specified
        if not self.webdriver_binary:
            self.webdriver_binary = self._find_webdriver_binary()

        if not self.binary:
            self.binary = self._find_browser_binary()

    def _find_webdriver_binary(self) -> str:
        """Find the WebDriver server binary"""
        # Check common locations
        paths = [
            "target/release/webdriver-server",
            "target/debug/webdriver-server",
            "./webdriver-server",
        ]

        for path in paths:
            if os.path.exists(path):
                return os.path.abspath(path)

        # Default to release build
        return "target/release/webdriver-server"

    def _find_browser_binary(self) -> str:
        """Find the browser binary"""
        # For now, FrankenBrowser WebDriver runs standalone
        # In future, this would point to the actual browser executable
        return None

    def setup(self, **kwargs):
        """Setup before running tests"""
        pass

    def start(self, **kwargs):
        """Start the WebDriver server"""
        print(f"Starting FrankenBrowser WebDriver server on port {self.webdriver_port}")

        # Build the binary if it doesn't exist
        if not os.path.exists(self.webdriver_binary):
            print("WebDriver binary not found, building...")
            subprocess.run([
                "cargo", "build", "--release", "--bin", "webdriver-server"
            ], check=True)

        # Start WebDriver server
        self.process = subprocess.Popen(
            [self.webdriver_binary] + self.webdriver_args,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        # Wait for server to be ready
        time.sleep(2)

        # Verify server is running
        if self.process.poll() is not None:
            stdout, stderr = self.process.communicate()
            raise RuntimeError(
                f"WebDriver server failed to start.\n"
                f"stdout: {stdout.decode()}\n"
                f"stderr: {stderr.decode()}"
            )

        print(f"WebDriver server started (PID: {self.process.pid})")
        return True

    def stop(self, **kwargs):
        """Stop the WebDriver server"""
        if self.process:
            print(f"Stopping WebDriver server (PID: {self.process.pid})")
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                print("WebDriver server didn't stop gracefully, killing...")
                self.process.kill()
                self.process.wait()

            self.process = None

    def cleanup(self, **kwargs):
        """Cleanup after tests"""
        self.stop()

    def executor_kwargs(self, test_type, test_environment, run_info_data, **kwargs) -> Dict[str, Any]:
        """Return kwargs for the test executor"""
        executor_kwargs = {
            "host": "127.0.0.1",
            "port": self.webdriver_port,
            "capabilities": {
                "browserName": "frankenbrowser",
                "browserVersion": "0.1.0",
                "platformName": "linux",
            },
        }

        # Add test-type specific configuration
        if test_type == "testharness":
            executor_kwargs["timeout_multiplier"] = kwargs.get("timeout_multiplier", 1)

        return executor_kwargs

    def get_version(self) -> str:
        """Get browser version"""
        return "0.1.0"

    def get_browser_cls(self, test_type):
        """Get the browser class for a test type"""
        # WPT will use WebDriver protocol
        from wptrunner.browsers.base import Browser
        return Browser


# Register the product with WPT
def register_product():
    """Register FrankenBrowser with WPT"""
    from wptrunner import products

    products.products["frankenbrowser"] = {
        "product": "frankenbrowser",
        "browser": "FrankenBrowser",
        "browser_cls": "wptrunner.browsers.base.Browser",
        "executor": {
            "testharness": "wptrunner.executors.executorwebdriver.WebDriverTestharnessExecutor",
            "reftest": "wptrunner.executors.executorwebdriver.WebDriverRefTestExecutor",
        },
        "product_cls": FrankenBrowserProduct,
    }


if __name__ == "__main__":
    # Test the adapter
    print("Testing FrankenBrowser WPT adapter...")

    adapter = FrankenBrowserProduct()
    print(f"WebDriver binary: {adapter.webdriver_binary}")
    print(f"Browser binary: {adapter.binary}")

    try:
        adapter.start()
        print("✓ WebDriver server started successfully")
        time.sleep(2)
        print("✓ Server is running")
    finally:
        adapter.stop()
        print("✓ Server stopped successfully")

    print("\nAdapter test complete!")
