use crate::config::MULLVAD_INTERFACE_NAME;

use super::{
    super::stats::{Stats, StatsMap},
    Config, Error as WgKernelError, Handle, Tunnel, TunnelError,
};
use futures::Future;
use std::{collections::HashMap, pin::Pin};
use talpid_dbus::{
    dbus,
    network_manager::{
        DeviceConfig, Error as NetworkManagerError, NetworkManager, Variant, VariantMap,
        WireguardTunnel,
    },
};
use talpid_net::unix::iface_index;
use talpid_tunnel_config_client::DaitaSettings;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error while communicating over Dbus")]
    Dbus(#[from] dbus::Error),

    #[error("NetworkManager error")]
    NetworkManager(#[from] NetworkManagerError),
}

pub struct NetworkManagerTunnel {
    network_manager: NetworkManager,
    tunnel: Option<WireguardTunnel>,
    netlink_connections: Handle,
    interface_name: String,
}

impl NetworkManagerTunnel {
    pub fn new(
        tokio_handle: tokio::runtime::Handle,
        config: &Config,
    ) -> std::result::Result<Self, WgKernelError> {
        let network_manager = NetworkManager::new()
            .map_err(Error::NetworkManager)
            .map_err(WgKernelError::NetworkManager)?;
        let config_map = convert_config_to_dbus(config);
        let tunnel = network_manager
            .create_wg_tunnel(&config_map)
            .map_err(|err| WgKernelError::NetworkManager(err.into()))?;

        let interface_name = match network_manager.get_interface_name(&tunnel) {
            Ok(name) => name,
            Err(error) => {
                log::error!("Failed to fetch interface name from NM: {}", error);
                MULLVAD_INTERFACE_NAME.to_string()
            }
        };
        let netlink_connections = tokio_handle.block_on(Handle::connect())?;

        Ok(NetworkManagerTunnel {
            network_manager,
            tunnel: Some(tunnel),
            netlink_connections,
            interface_name,
        })
    }
}

#[async_trait::async_trait]
impl Tunnel for NetworkManagerTunnel {
    fn get_interface_name(&self) -> String {
        self.interface_name.clone()
    }

    fn stop(mut self: Box<Self>) -> std::result::Result<(), TunnelError> {
        if let Some(tunnel) = self.tunnel.take() {
            if let Err(err) = self.network_manager.remove_tunnel(tunnel) {
                log::error!("Failed to remove WireGuard tunnel via NM: {}", err);
                Err(TunnelError::StopWireguardError(Box::new(err)))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    async fn get_tunnel_stats(&self) -> std::result::Result<StatsMap, TunnelError> {
        let mut wg = self.netlink_connections.wg_handle.clone();
        let device = wg
            .get_by_name(self.interface_name.clone())
            .await
            .map_err(|err| {
                log::error!("Failed to fetch WireGuard device config: {}", err);
                TunnelError::GetConfigError
            })?;
        Ok(Stats::parse_device_message(&device))
    }

    fn set_config(
        &mut self,
        config: Config,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), TunnelError>> + Send>> {
        let interface_name = self.interface_name.clone();
        let mut wg = self.netlink_connections.wg_handle.clone();
        Box::pin(async move {
            let index = iface_index(&interface_name).map_err(|err| {
                log::error!("Failed to fetch WireGuard device index: {}", err);
                TunnelError::SetConfigError
            })?;
            wg.set_config(index, &config).await.map_err(|err| {
                log::error!("Failed to apply WireGuard config: {}", err);
                TunnelError::SetConfigError
            })
        })
    }

    /// Outright fail to start - this tunnel type does not support DAITA.
    fn start_daita(&mut self, _: DaitaSettings) -> std::result::Result<(), TunnelError> {
        Err(TunnelError::DaitaNotSupported)
    }
}

fn convert_config_to_dbus(config: &Config) -> DeviceConfig {
    let mut ipv6_config: VariantMap = HashMap::new();
    let mut ipv4_config: VariantMap = HashMap::new();
    let mut wireguard_config: VariantMap = HashMap::new();
    let mut connection_config: VariantMap = HashMap::new();
    let mut peer_configs = vec![];

    wireguard_config.insert("mtu".into(), Variant(Box::new(config.mtu as u32)));
    if let Some(fwmark) = config.fwmark {
        wireguard_config.insert("fwmark".into(), Variant(Box::new(fwmark)));
    }
    wireguard_config.insert("peer-routes".into(), Variant(Box::new(false)));
    wireguard_config.insert(
        "private-key".into(),
        Variant(Box::new(config.tunnel.private_key.to_base64())),
    );
    wireguard_config.insert("private-key-flags".into(), Variant(Box::new(0x0u32)));

    for peer in config.peers() {
        let mut peer_config: VariantMap = HashMap::new();
        let allowed_ips = peer
            .allowed_ips
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        peer_config.insert("allowed-ips".into(), Variant(Box::new(allowed_ips)));
        peer_config.insert(
            "endpoint".into(),
            Variant(Box::new(peer.endpoint.to_string())),
        );
        peer_config.insert(
            "public-key".into(),
            Variant(Box::new(peer.public_key.to_base64())),
        );

        peer_configs.push(peer_config);
    }
    wireguard_config.insert("peers".into(), Variant(Box::new(peer_configs)));

    connection_config.insert("type".into(), Variant(Box::new("wireguard".to_string())));
    connection_config.insert(
        "id".into(),
        Variant(Box::new(MULLVAD_INTERFACE_NAME.to_string())),
    );
    connection_config.insert(
        "interface-name".into(),
        Variant(Box::new(MULLVAD_INTERFACE_NAME.to_string())),
    );
    connection_config.insert("autoconnect".into(), Variant(Box::new(true)));

    let ipv4_addrs: Vec<_> = config
        .tunnel
        .addresses
        .iter()
        .filter(|ip| ip.is_ipv4())
        .map(NetworkManager::convert_address_to_dbus)
        .collect();

    let ipv6_addrs: Vec<_> = config
        .tunnel
        .addresses
        .iter()
        .filter(|ip| ip.is_ipv6())
        .map(NetworkManager::convert_address_to_dbus)
        .collect();

    ipv4_config.insert("address-data".into(), Variant(Box::new(ipv4_addrs)));
    ipv4_config.insert("ignore-auto-routes".into(), Variant(Box::new(true)));
    ipv4_config.insert("ignore-auto-dns".into(), Variant(Box::new(true)));
    ipv4_config.insert("may-fail".into(), Variant(Box::new(true)));
    ipv4_config.insert("method".into(), Variant(Box::new("manual".to_string())));
    ipv4_config.insert("never-default".into(), Variant(Box::new(true)));

    if !ipv6_addrs.is_empty() {
        ipv6_config.insert("method".into(), Variant(Box::new("manual".to_string())));
        ipv6_config.insert("address-data".into(), Variant(Box::new(ipv6_addrs)));
        ipv6_config.insert("ignore-auto-routes".into(), Variant(Box::new(true)));
        ipv6_config.insert("ignore-auto-dns".into(), Variant(Box::new(true)));
        ipv6_config.insert("may-fail".into(), Variant(Box::new(true)));
    }

    let mut settings = HashMap::new();
    settings.insert("ipv4".into(), ipv4_config);
    if !ipv6_config.is_empty() {
        settings.insert("ipv6".into(), ipv6_config);
    }
    settings.insert("wireguard".into(), wireguard_config);
    settings.insert("connection".into(), connection_config);

    settings
}
