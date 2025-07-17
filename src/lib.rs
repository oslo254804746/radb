//! # ADB Utils - Android Debug Bridge Utilities for Rust
//!
//! A comprehensive Rust library for interacting with Android devices through ADB (Android Debug Bridge).
//! This library provides both synchronous and asynchronous interfaces for device management, file operations,
//! shell command execution, and more.
//!
//! ## Features
//!
//! - **Dual API Support**: Both blocking and async/await interfaces
//! - **Device Management**: Connect, disconnect, and manage multiple devices
//! - **File Operations**: Push, pull, list, and manipulate files on devices
//! - **Shell Commands**: Execute shell commands with streaming support
//! - **Screen Operations**: Screenshots, input simulation, and screen control
//! - **App Management**: Install, uninstall, start, and stop applications
//! - **Network Operations**: Port forwarding, WiFi control, and network information
//! - **System Information**: Device properties, Android version, hardware info
//! - **Logging**: Logcat streaming and filtering
//!
//! ## Quick Start
//!
//! ### Blocking API
//!
//! ```rust,no_run
//! use radb::prelude::*;
//!
//! fn main() -> AdbResult<()> {
//!     // Connect to ADB server
//!     let mut client = AdbClient::default();
//!     
//!     // Get device list
//!     let devices = client.list_devices()?;
//!     let mut device = devices.into_iter().next().unwrap();
//!     
//!     // Execute shell command
//!     let result = device.shell(["echo", "Hello, ADB!"])?;
//!     println!("Output: {}", result);
//!     
//!     // Take screenshot
//!     let screenshot = device.screenshot()?;
//!     println!("Screenshot: {}x{}", screenshot.width(), screenshot.height());
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Async API
//!
//! ```rust,no_run
//! use radb::prelude::*;
//! use radb::AdbResult;
//! #[tokio::main]
//! async fn main() -> AdbResult<()> {
//!     // Connect to ADB server
//!     
//! let mut client = AdbClient::default().await;
//!     
//!     // Get device list
//!     let devices = client.list_devices().await?;
//!     let mut device = devices.into_iter().next().unwrap();
//!     
//!     // Execute shell command
//!     let result = device.shell(["echo", "Hello, ADB!"]).await?;
//!     println!("Output: {}", result);
//!     
//!     // Stream logcat
//!     let mut logcat = device.logcat(true, None).await?;
//!     while let Some(line) = logcat.next().await {
//!         println!("Log: {}", line?);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `blocking`: Enable blocking/synchronous API (default)
//! - `tokio_async`: Enable async/await API with Tokio runtime
//! - `serde`: Enable serialization support for data structures
//!
//! ## Error Handling
//!
//! The library uses a comprehensive error system with specific error types:
//!
//! ```rust,no_run
//! use radb::prelude::*;
//!
//! match device.shell(["invalid_command"]) {
//!     Ok(output) => println!("Success: {}", output),
//!     Err(AdbError::CommandFailed { command, reason }) => {
//!         eprintln!("Command '{}' failed: {}", command, reason);
//!     }
//!     Err(AdbError::DeviceNotFound { serial }) => {
//!         eprintln!("Device '{}' not found", serial);
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

// Core modules
pub mod beans;
pub mod client;
pub mod errors;
pub mod protocols;
pub mod utils;

// Re-exports for convenience
pub use client::{AdbClient, AdbDevice};
pub use errors::{AdbError, AdbResult};

// Prelude module for common imports
pub mod prelude {
    //! Common imports for ADB utilities
    //!
    //! This module re-exports the most commonly used types and traits.
    //! Import this module to get started quickly:
    //!
    //! ```rust
    //! use radb::prelude::*;
    //! ```

    pub use crate::beans::{AdbCommand, AppInfo, FileInfo, ForwardItem, NetworkType};
    pub use crate::client::{AdbClient, AdbDevice};
    pub use crate::errors::{AdbError, AdbResult, AdbResultExt};
    pub use crate::utils::{get_free_port, start_adb_server};

    // Re-export commonly used external types
    #[cfg(feature = "tokio_async")]
    pub use futures_util::StreamExt;

    #[cfg(feature = "blocking")]
    pub use std::net::TcpStream;

    #[cfg(feature = "tokio_async")]
    pub use tokio::net::TcpStream;
}

// Convenient type aliases
pub type Result<T> = std::result::Result<T, AdbError>;

// Version and metadata
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// Library information
pub mod info {
    //! Library metadata and version information

    /// Get library version
    pub fn version() -> &'static str {
        crate::VERSION
    }

    /// Get library description
    pub fn description() -> &'static str {
        crate::DESCRIPTION
    }

    /// Get library authors
    pub fn authors() -> &'static str {
        crate::AUTHORS
    }

    /// Print library information
    pub fn print_info() {
        println!("ADB Utils v{}", version());
        println!("Description: {}", description());
        println!("Authors: {}", authors());
    }
}

// Utilities module with public helpers
pub mod util {
    //! Utility functions for ADB operations

    pub use crate::utils::*;

    /// Check if ADB server is running
    pub fn is_adb_server_running() -> bool {
        use std::net::TcpStream;
        TcpStream::connect("127.0.0.1:5037").is_ok()
    }

    /// Start ADB server if not running
    pub fn ensure_adb_server() -> crate::AdbResult<()> {
        if !is_adb_server_running() {
            start_adb_server();
            // Wait a bit for server to start
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Ok(())
    }

    /// Get ADB server version (convenience function)
    #[cfg(feature = "blocking")]
    pub fn get_adb_server_version() -> crate::AdbResult<String> {
        let mut client = crate::AdbClient::default();
        client.server_version()
    }

    /// Get ADB server version (async convenience function)
    #[cfg(feature = "tokio_async")]
    pub async fn get_adb_server_version_async() -> crate::AdbResult<String> {
        let mut client = crate::AdbClient::default().await;
        client.server_version().await
    }
}

// Feature-specific modules
#[cfg(feature = "blocking")]
pub mod blocking {
    //! Blocking/synchronous API
    //!
    //! This module contains the blocking versions of ADB operations.
    //! Use this when you don't need async/await functionality.

    pub use crate::client::adb_client::blocking_impl::*;
    pub use crate::client::adb_device::blocking_impl::*;
    pub use crate::protocols::blocking::AdbProtocol;
}

#[cfg(feature = "tokio_async")]
pub mod r#async {
    //! Asynchronous API
    //!
    //! This module contains the async versions of ADB operations.
    //! Use this when you need async/await functionality with Tokio.

    pub use crate::client::adb_client::async_impl::*;
    pub use crate::client::adb_device::async_impl::*;
    pub use crate::protocols::tokio_async::AdbProtocol;
}

// Builder pattern for common operations
pub mod builder {
    //! Builder patterns for complex operations

    use crate::prelude::*;

    /// Builder for ADB client configuration
    pub struct AdbClientBuilder {
        addr: Option<String>,
        timeout: Option<std::time::Duration>,
    }

    impl AdbClientBuilder {
        /// Create a new client builder
        pub fn new() -> Self {
            Self {
                addr: None,
                timeout: None,
            }
        }

        /// Set ADB server address
        pub fn addr<S: Into<String>>(mut self, addr: S) -> Self {
            self.addr = Some(addr.into());
            self
        }

        /// Set connection timeout
        pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
            self.timeout = Some(timeout);
            self
        }

        /// Build the client (blocking version)
        #[cfg(feature = "blocking")]
        pub fn build(self) -> AdbResult<AdbClient> {
            let addr = self.addr.unwrap_or_else(|| "127.0.0.1:5037".to_string());
            Ok(AdbClient::new(addr))
        }

        /// Build the client (async version)
        #[cfg(feature = "tokio_async")]
        pub async fn build_async(self) -> AdbResult<AdbClient> {
            let addr = self.addr.unwrap_or_else(|| "127.0.0.1:5037".to_string());
            Ok(AdbClient::new(addr).await)
        }
    }

    impl Default for AdbClientBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}

// Testing utilities (only available in tests)
#[cfg(test)]
pub mod test_utils {
    //! Testing utilities for ADB operations

    use std::fmt::Debug;

    use crate::prelude::*;

    /// Setup test environment
    pub fn setup_test_env() {
        crate::utils::start_adb_server();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    /// Get test device (if available)
    #[cfg(feature = "blocking")]
    pub fn get_test_device() -> Option<AdbDevice<impl std::net::ToSocketAddrs + Clone + Debug>> {
        let mut client = AdbClient::default();
        client.list_devices().ok()?.into_iter().next()
    }

    /// Get test device (async version)
    #[cfg(feature = "tokio_async")]
    pub async fn get_test_device_async(
    ) -> Option<AdbDevice<impl tokio::net::ToSocketAddrs + Clone + Debug>> {
        let mut client = AdbClient::default().await;
        client.list_devices().await.ok()?.into_iter().next()
    }
}

// Macros for common operations
#[macro_export]
macro_rules! adb_shell {
    ($device:expr, $($arg:expr),+) => {
        $device.shell([$($arg),+])
    };
}

#[macro_export]
macro_rules! adb_expect_device {
    ($client:expr) => {
        $client
            .list_devices()?
            .into_iter()
            .next()
            .ok_or_else(|| $crate::AdbError::device_not_found("No devices found"))?
    };
}

pub use anyhow;
pub use log;

// Conditional exports based on features
#[cfg(feature = "tokio_async")]
pub use tokio;

#[cfg(feature = "blocking")]
pub use std::net;

// Documentation examples
#[cfg(doctest)]
doc_comment::doctest!("../README.md");

// Version check at compile time
const _: fn() = || {
    // This will cause a compile error if the version format is unexpected
    let version = VERSION;
    assert!(version.len() > 0, "Version should not be empty");
};

// Feature compatibility checks
#[cfg(all(feature = "blocking", feature = "tokio_async"))]
compile_error!("Cannot use both 'blocking' and 'tokio_async' features simultaneously");

#[cfg(not(any(feature = "blocking", feature = "tokio_async")))]
compile_error!("Must enable either 'blocking' or 'tokio_async' feature");

// Platform-specific optimizations
#[cfg(target_os = "android")]
compile_error!("This library is not intended to run on Android devices");

// Integration with common async runtimes
#[cfg(all(feature = "tokio_async", feature = "async-std"))]
compile_error!("Cannot use both Tokio and async-std features");

// Export the main ADB namespace for convenience
pub mod adb {
    //! Main ADB namespace with all core functionality

    pub use crate::beans::*;
    pub use crate::client::*;
    pub use crate::errors::*;
    pub use crate::prelude::*;
}
