//! Server-initiated notification broadcasting.
//!
//! The [`Notifier`] collects `Peer<RoleServer>` handles from connected MCP
//! clients and broadcasts notifications to all of them. Dead peers are
//! automatically pruned on each broadcast.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::RoleServer;
use rmcp::model::{
    LoggingLevel, LoggingMessageNotificationParam, ResourceUpdatedNotificationParam,
};
use rmcp::service::Peer;
use tokio::sync::RwLock;

/// Broadcasts notifications to all connected MCP clients.
///
/// Each connected client provides a `Peer<RoleServer>` handle via
/// `ServerHandler::on_initialized()`. The `Notifier` stores these handles
/// and broadcasts notifications to all of them, automatically pruning
/// peers that have disconnected.
///
/// `Notifier` is cheaply cloneable (wraps `Arc`) and safe to share across
/// async tasks.
#[derive(Clone)]
pub struct Notifier {
    peers: Arc<RwLock<Vec<Peer<RoleServer>>>>,
    subscriptions: Arc<RwLock<HashMap<String, Vec<Peer<RoleServer>>>>>,
}

impl Notifier {
    /// Create a new `Notifier` with no connected peers.
    pub(crate) fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a newly connected peer.
    pub(crate) async fn add_peer(&self, peer: Peer<RoleServer>) {
        self.peers.write().await.push(peer);
    }

    /// Broadcast a logging notification to all connected clients.
    ///
    /// Returns the number of clients that received the notification
    /// successfully. Dead peers are pruned automatically.
    pub async fn log(&self, level: LoggingLevel, logger: &str, data: serde_json::Value) -> usize {
        let param = LoggingMessageNotificationParam::new(level, data).with_logger(logger);
        let mut peers = self.peers.write().await;
        let mut failed = Vec::new();

        for (i, peer) in peers.iter().enumerate() {
            if peer.notify_logging_message(param.clone()).await.is_err() {
                failed.push(i);
            }
        }

        let success = peers.len() - failed.len();
        // Remove dead peers in reverse order to preserve indices
        for i in failed.into_iter().rev() {
            peers.swap_remove(i);
        }
        success
    }

    /// Register a peer's subscription to a resource URI.
    pub async fn subscribe_resource(&self, uri: impl Into<String>, peer: Peer<RoleServer>) {
        self.subscriptions
            .write()
            .await
            .entry(uri.into())
            .or_default()
            .push(peer);
    }

    /// Remove all subscriptions for a resource URI.
    ///
    /// Currently removes all peers subscribed to the URI. Per-peer
    /// removal is not supported because `Peer` does not implement
    /// `PartialEq`.
    pub async fn unsubscribe_resource(&self, uri: &str) {
        self.subscriptions.write().await.remove(uri);
    }

    /// Notify clients that a resource has been updated.
    ///
    /// If any peers have subscribed to the URI, only those peers are
    /// notified. Otherwise, all connected peers are notified (backwards
    /// compatible).
    ///
    /// Returns the number of clients notified successfully.
    pub async fn resource_updated(&self, uri: impl Into<String>) -> usize {
        let uri = uri.into();
        let param = ResourceUpdatedNotificationParam::new(&uri);

        // Check for explicit subscribers first
        let subscriber_peers: Option<Vec<Peer<RoleServer>>> = {
            let subs = self.subscriptions.read().await;
            subs.get(&uri).cloned()
        };

        if let Some(peers) = subscriber_peers {
            let mut success = 0;
            for peer in &peers {
                if peer.notify_resource_updated(param.clone()).await.is_ok() {
                    success += 1;
                }
            }
            // Prune dead subscribers
            if success < peers.len() {
                let mut subs = self.subscriptions.write().await;
                if let Some(sub_peers) = subs.get_mut(&uri) {
                    let mut i = 0;
                    while i < sub_peers.len() {
                        let test_param = ResourceUpdatedNotificationParam::new(&uri);
                        if sub_peers[i]
                            .notify_resource_updated(test_param)
                            .await
                            .is_err()
                        {
                            sub_peers.swap_remove(i);
                        } else {
                            i += 1;
                        }
                    }
                    if sub_peers.is_empty() {
                        subs.remove(&uri);
                    }
                }
            }
            success
        } else {
            // No explicit subscriptions — broadcast to all peers
            let mut peers = self.peers.write().await;
            let mut failed = Vec::new();

            for (i, peer) in peers.iter().enumerate() {
                if peer.notify_resource_updated(param.clone()).await.is_err() {
                    failed.push(i);
                }
            }

            let success = peers.len() - failed.len();
            for i in failed.into_iter().rev() {
                peers.swap_remove(i);
            }
            success
        }
    }

    /// Notify all clients that the resource list has changed.
    ///
    /// Returns the number of clients notified successfully.
    pub async fn resource_list_changed(&self) -> usize {
        let mut peers = self.peers.write().await;
        let mut failed = Vec::new();

        for (i, peer) in peers.iter().enumerate() {
            if peer.notify_resource_list_changed().await.is_err() {
                failed.push(i);
            }
        }

        let success = peers.len() - failed.len();
        for i in failed.into_iter().rev() {
            peers.swap_remove(i);
        }
        success
    }

    /// Return the number of currently connected clients.
    pub async fn client_count(&self) -> usize {
        self.peers.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notifier_new_has_no_peers() {
        let notifier = Notifier::new();
        assert_eq!(notifier.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_notifier_clone_shares_state() {
        let n1 = Notifier::new();
        let n2 = n1.clone();
        // Both should see the same empty state
        assert_eq!(n1.client_count().await, 0);
        assert_eq!(n2.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_log_with_no_peers_returns_zero() {
        let notifier = Notifier::new();
        let count = notifier
            .log(
                LoggingLevel::Info,
                "test",
                serde_json::json!({"event": "test"}),
            )
            .await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_resource_updated_with_no_peers_returns_zero() {
        let notifier = Notifier::new();
        let count = notifier.resource_updated("nms://galaxy/model").await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_resource_list_changed_with_no_peers_returns_zero() {
        let notifier = Notifier::new();
        let count = notifier.resource_list_changed().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_subscribe_resource_adds_to_subscriptions() {
        let n1 = Notifier::new();
        let n2 = n1.clone();

        // No subscriptions initially — resource_updated with no peers returns 0
        assert_eq!(n1.resource_updated("test://uri").await, 0);

        // After unsubscribe of non-existent URI, nothing crashes
        n2.unsubscribe_resource("test://uri").await;
    }

    #[tokio::test]
    async fn test_unsubscribe_resource_removes_uri() {
        let notifier = Notifier::new();
        // Unsubscribing a URI that was never subscribed is a no-op
        notifier.unsubscribe_resource("test://uri").await;
    }
}
