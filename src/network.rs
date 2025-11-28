//! Network change detection module using D-Bus NetworkManager
//! Monitors connectivity state and VPN changes to trigger location refresh.

use futures_util::StreamExt;
use thiserror::Error;
use tokio::sync::mpsc;
use zbus::{proxy, Connection, zvariant::OwnedObjectPath};

/// NetworkManager connectivity states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NetworkState {
    Unknown = 0,
    Asleep = 10,
    Disconnected = 20,
    Disconnecting = 30,
    Connecting = 40,
    ConnectedLocal = 50,
    ConnectedSite = 60,
    ConnectedGlobal = 70,
}

impl From<u32> for NetworkState {
    fn from(value: u32) -> Self {
        match value {
            0 => NetworkState::Unknown,
            10 => NetworkState::Asleep,
            20 => NetworkState::Disconnected,
            30 => NetworkState::Disconnecting,
            40 => NetworkState::Connecting,
            50 => NetworkState::ConnectedLocal,
            60 => NetworkState::ConnectedSite,
            70 => NetworkState::ConnectedGlobal,
            _ => NetworkState::Unknown,
        }
    }
}

impl NetworkState {
    /// Returns true if we have full internet connectivity
    pub fn is_connected(&self) -> bool {
        matches!(self, NetworkState::ConnectedGlobal)
    }
}

/// Events emitted by network monitor
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Network is now connected with internet access
    Connected,
    /// Network disconnected or connectivity lost
    Disconnected,
}

/// Errors during network monitoring
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("D-Bus connection failed: {0}")]
    Connection(#[from] zbus::Error),
    #[error("Channel send failed")]
    ChannelClosed,
}

/// D-Bus proxy for NetworkManager
#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    /// Get current connectivity state
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// Get active connections (changes when VPN connects/disconnects)
    #[zbus(property)]
    fn active_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

/// Watches for network connectivity changes via NetworkManager D-Bus interface.
/// Monitors both connectivity state and active connections (for VPN changes).
pub async fn watch_network_changes(tx: mpsc::Sender<NetworkEvent>) -> Result<(), NetworkError> {
    let connection = Connection::system().await?;
    let proxy = NetworkManagerProxy::new(&connection).await?;

    // Get initial state
    let initial_state = NetworkState::from(proxy.state().await.unwrap_or(0));
    let mut was_connected = initial_state.is_connected();
    let mut last_connections = proxy.active_connections().await.unwrap_or_default().len();

    tracing::info!("Initial network state: {:?} (connected={})", initial_state, was_connected);

    // Watch for state changes
    let mut state_stream = proxy.receive_state_changed().await;
    // Watch for active connection changes (VPN connect/disconnect)
    let mut conn_stream = proxy.receive_active_connections_changed().await;

    loop {
        tokio::select! {
            Some(change) = state_stream.next() => {
                if let Ok(new_val) = change.get().await {
                    let new_state = NetworkState::from(new_val);
                    let is_connected = new_state.is_connected();

                    tracing::debug!("Network state changed: {:?} (connected={})", new_state, is_connected);

                    if is_connected != was_connected {
                        let event = if is_connected {
                            NetworkEvent::Connected
                        } else {
                            NetworkEvent::Disconnected
                        };
                        if tx.send(event).await.is_err() {
                            return Err(NetworkError::ChannelClosed);
                        }
                        was_connected = is_connected;
                    }
                }
            }
            Some(change) = conn_stream.next() => {
                // Active connections changed (VPN connect/disconnect)
                if let Ok(connections) = change.get().await {
                    let new_count = connections.len();
                    if new_count != last_connections && was_connected {
                        tracing::info!("Active connections changed: {} -> {} (VPN change detected)", last_connections, new_count);
                        // Emit Connected event to trigger refresh
                        if tx.send(NetworkEvent::Connected).await.is_err() {
                            return Err(NetworkError::ChannelClosed);
                        }
                    }
                    last_connections = new_count;
                }
            }
        }
    }
}
