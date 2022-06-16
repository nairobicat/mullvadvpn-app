use crate::{
    location::{CityCode, CountryCode, Location},
};
#[cfg(target_os = "android")]
use jnix::IntoJava;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};
use talpid_types::net::{
    openvpn::{ProxySettings, ShadowsocksProxySettings},
    wireguard, TransportProtocol,
};

/// Stores a list of relays for each country obtained from the API using
/// `mullvad_api::RelayListProxy`. This can also be passed to frontends.
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
pub struct RelayList {
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub etag: Option<String>,
    pub countries: Vec<RelayListCountry>,
    #[cfg_attr(target_os = "android", jnix(skip))]
    #[serde(rename = "openvpn")]
    pub openvpn: OpenVpnEndpointData,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub bridges: BridgeEndpointData,
    pub wireguard: WireguardEndpointData,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub obfuscators: ObfuscatorEndpointData,
}

impl RelayList {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// A list of [`RelayListCity`]s within a country. Used by [`RelayList`].
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
pub struct RelayListCountry {
    pub name: String,
    pub code: CountryCode,
    pub cities: Vec<RelayListCity>,
}

/// A list of [`Relay`]s within a city. Used by [`RelayListCountry`].
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
pub struct RelayListCity {
    pub name: String,
    pub code: CityCode,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub latitude: f64,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub longitude: f64,
    pub relays: Vec<Relay>,
}

/// Stores information for a relay returned by the API at `v1/relays` using
/// `mullvad_api::RelayListProxy`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
pub struct Relay {
    pub hostname: String,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub ipv4_addr_in: Ipv4Addr,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub ipv6_addr_in: Option<Ipv6Addr>,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub include_in_country: bool,
    pub active: bool,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub owned: bool,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub provider: String,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub weight: u64,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub endpoint_data: RelayEndpointData,
    #[cfg_attr(target_os = "android", jnix(skip))]
    pub location: Option<Location>,
}

/// Specifies the type of a relay or relay-specific endpoint data.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayEndpointData {
    Openvpn,
    Bridge,
    Wireguard(WireguardRelayEndpointData),
}

/// Data needed to connect to OpenVPN endpoints.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct OpenVpnEndpointData {
    pub ports: Vec<OpenVpnEndpoint>,
}

/// Data needed to connect to OpenVPN endpoints.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct OpenVpnEndpoint {
    pub port: u16,
    pub protocol: TransportProtocol,
}

// FIXME
/*
impl OpenVpnEndpointData {
    pub fn into_mullvad_endpoint(self, host: IpAddr) -> MullvadEndpoint {
        MullvadEndpoint::OpenVpn(Endpoint::new(host, self.port, self.protocol))
    }
}

impl fmt::Display for OpenVpnEndpointData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{} port {}", self.protocol, self.port)
    }
}
*/

/// Contains data about all WireGuard endpoints, such as valid port ranges.
#[derive(Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Debug)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
#[cfg_attr(target_os = "android", jnix(skip_all))]
pub struct WireguardEndpointData {
    /// Port to connect to
    pub port_ranges: Vec<(u16, u16)>,
    /// Gateways to be used with the tunnel
    pub ipv4_gateway: Ipv4Addr,
    pub ipv6_gateway: Ipv6Addr,
}

impl Default for WireguardEndpointData {
    fn default() -> Self {
        Self {
            port_ranges: vec![],
            ipv4_gateway: "0.0.0.0".parse().unwrap(),
            ipv6_gateway: "::".parse().unwrap(),
        }
    }
}

/// Contains data about specific WireGuard endpoints, i.e. their public keys.
#[derive(Clone, Eq, PartialEq, Hash, Deserialize, Serialize, Debug)]
#[cfg_attr(target_os = "android", derive(IntoJava))]
#[cfg_attr(target_os = "android", jnix(package = "net.mullvad.mullvadvpn.model"))]
#[cfg_attr(target_os = "android", jnix(skip_all))]
pub struct WireguardRelayEndpointData {
    /// Public key used by the relay peer
    pub public_key: wireguard::PublicKey,
}

// FIXME
/*
impl fmt::Display for WireguardEndpointData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "gateways {} - {} port_ranges {{ {} }} public_key {}",
            self.ipv4_gateway,
            self.ipv6_gateway,
            self.port_ranges
                .iter()
                .map(|range| format!("[{} - {}]", range.0, range.1))
                .collect::<Vec<_>>()
                .join(","),
            self.public_key,
        )
    }
}
*/

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BridgeEndpointData {
    pub shadowsocks: Vec<ShadowsocksEndpointData>,
}

/// Data needed to connect to Shadowsocks endpoints.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ShadowsocksEndpointData {
    pub port: u16,
    pub cipher: String,
    pub password: String,
    pub protocol: TransportProtocol,
}

impl ShadowsocksEndpointData {
    pub fn to_proxy_settings(&self, addr: IpAddr) -> ProxySettings {
        ProxySettings::Shadowsocks(ShadowsocksProxySettings {
            peer: SocketAddr::new(addr, self.port),
            password: self.password.clone(),
            cipher: self.cipher.clone(),
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ObfuscatorEndpointData {
    pub udp2tcp: Vec<Udp2TcpEndpointData>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Udp2TcpEndpointData {
    pub port: u16,
}
