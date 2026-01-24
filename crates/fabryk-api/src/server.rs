//! API server implementation

use crate::Result;

/// Fabryk API server
pub struct Server;

impl Server {
    /// Create a new server instance
    pub fn new() -> Result<Self> {
        // TODO: Implement server initialization
        Ok(Self)
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new().expect("Failed to create default server")
    }
}
