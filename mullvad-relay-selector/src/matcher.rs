use mullvad_types::{
    endpoint::{MullvadEndpoint, MullvadWireguardEndpoint},
    relay_constraints::{
        Constraint, LocationConstraint, Match, OpenVpnConstraints, Ownership, Providers,
        RelayConstraints, WireguardConstraints,
    },
    relay_list::{Relay, RelayEndpointData, OpenVpnEndpointData, WireguardEndpointData},
};
use rand::Rng;
use std::net::{IpAddr, SocketAddr};
use talpid_types::net::{all_of_the_internet, wireguard, Endpoint, IpVersion, TransportProtocol, TunnelType};

#[derive(Clone)]
pub struct RelayMatcher<T: TunnelMatcher> {
    pub location: Constraint<LocationConstraint>,
    pub providers: Constraint<Providers>,
    pub ownership: Constraint<Ownership>,
    pub tunnel: T,
}

impl RelayMatcher<AnyTunnelMatcher> {
    pub fn new(
        constraints: RelayConstraints,
        openvpn_data: OpenVpnEndpointData,
        wireguard_data: WireguardEndpointData,
    ) -> Self {
        Self {
            location: constraints.location,
            providers: constraints.providers,
            ownership: constraints.ownership,
            tunnel: AnyTunnelMatcher {
                wireguard: WireguardMatcher::new(constraints.wireguard_constraints, wireguard_data),
                openvpn: OpenVpnMatcher::new(constraints.openvpn_constraints, openvpn_data),
                tunnel_type: constraints.tunnel_protocol,
            },
        }
    }

    pub fn to_wireguard_matcher(self) -> RelayMatcher<WireguardMatcher> {
        RelayMatcher {
            tunnel: self.tunnel.wireguard,
            location: self.location,
            providers: self.providers,
            ownership: self.ownership,
        }
    }
}

impl RelayMatcher<WireguardMatcher> {
    pub fn set_peer(&mut self, peer: Relay) {
        self.tunnel.peer = Some(peer);
    }
}

impl<T: TunnelMatcher> RelayMatcher<T> {
    /// Filter a relay and its endpoints based on constraints.
    /// Only matching endpoints are included in the returned Relay.
    pub fn filter_matching_relay(&self, relay: &Relay) -> Option<Relay> {
        if !self.location.matches(relay)
            || !self.providers.matches(relay)
            || !self.ownership.matches(relay)
        {
            return None;
        }

        self.tunnel.filter_matching_endpoints(relay)
    }

    pub fn mullvad_endpoint(&self, relay: &Relay) -> Option<MullvadEndpoint> {
        self.tunnel.mullvad_endpoint(relay)
    }
}

/// TunnelMatcher allows to abstract over different tunnel-specific constraints,
/// as to not have false dependencies on OpenVpn specific constraints when
/// selecting only WireGuard tunnels.
pub trait TunnelMatcher: Clone {
    /// Filter a relay and its endpoints based on constraints.
    /// Only matching endpoints are included in the returned Relay.
    /// TODO: update desc here
    fn filter_matching_endpoints(&self, relay: &Relay) -> Option<Relay>;
    /// Constructs a MullvadEndpoint for a given Relay using extra data from the relay matcher
    /// itself.
    fn mullvad_endpoint(&self, relay: &Relay) -> Option<MullvadEndpoint>;
}

impl TunnelMatcher for OpenVpnMatcher {
    fn filter_matching_endpoints(&self, relay: &Relay) -> Option<Relay> {
        // FIXME: match against shared endpoint data
        if !matches!(relay.endpoint_data, RelayEndpointData::Openvpn) {
            return None;
        }
        Some(relay.clone())
    }

    fn mullvad_endpoint(&self, relay: &Relay) -> Option<MullvadEndpoint> {
        // FIXME: use shared endpoint data & pubkey
        Some(MullvadEndpoint::OpenVpn(Endpoint::new(
            relay.ipv4_addr_in,
            // FIXME: select a random port-protocol pair
            53,
            TransportProtocol::Udp,
        )))
    }
}

#[derive(Debug, Clone)]
pub struct OpenVpnMatcher {
    pub constraints: OpenVpnConstraints,
    pub data: OpenVpnEndpointData,
}

impl OpenVpnMatcher {
    pub fn new(constraints: OpenVpnConstraints, data: OpenVpnEndpointData) -> Self {
        Self { constraints, data }
    }
}

impl Match<OpenVpnEndpointData> for OpenVpnMatcher {
    fn matches(&self, endpoint: &OpenVpnEndpointData) -> bool {
        match self.constraints.port {
            Constraint::Any => true,
            Constraint::Only(transport_port) => endpoint
                .ports
                .iter()
                .any(|endpoint| {
                    transport_port.protocol == endpoint.protocol &&
                        (transport_port.port.is_any() ||
                            transport_port.port == Constraint::Only(endpoint.port))
                }),
        }
    }
}

#[derive(Clone)]
pub struct AnyTunnelMatcher {
    pub wireguard: WireguardMatcher,
    pub openvpn: OpenVpnMatcher,
    /// in the case that a user hasn't specified a tunnel protocol, the relay
    /// selector might still construct preferred constraints that do select a
    /// specific tunnel protocol, which is why the tunnel type may be specified
    /// in the `AnyTunnelMatcher`.
    pub tunnel_type: Constraint<TunnelType>,
}

impl TunnelMatcher for AnyTunnelMatcher {
    fn filter_matching_endpoints(&self, relay: &Relay) -> Option<Relay> {
        match self.tunnel_type {
            Constraint::Any => {
                let wireguard_relay = self.wireguard.filter_matching_endpoints(relay);
                let openvpn_relay = self.openvpn.filter_matching_endpoints(relay);

                match (wireguard_relay, openvpn_relay) {
                    (Some(relay), None) | (None, Some(relay)) => Some(relay),
                    (Some(_), Some(_)) => {
                        unreachable!("relay cannot match multiple endpoint types")
                    }
                    _ => None,
                }
            }
            Constraint::Only(TunnelType::OpenVpn) => self.openvpn.filter_matching_endpoints(relay),
            Constraint::Only(TunnelType::Wireguard) => {
                self.wireguard.filter_matching_endpoints(relay)
            }
        }
    }

    fn mullvad_endpoint(&self, relay: &Relay) -> Option<MullvadEndpoint> {
        #[cfg(not(target_os = "android"))]
        match self.tunnel_type {
            Constraint::Any => self.openvpn.mullvad_endpoint(relay).or_else(|| {
                self.wireguard.mullvad_endpoint(relay)
            }),
            Constraint::Only(TunnelType::OpenVpn) => self.openvpn.mullvad_endpoint(relay),
            Constraint::Only(TunnelType::Wireguard) => self.wireguard.mullvad_endpoint(relay),
        }

        #[cfg(target_os = "android")]
        self.wireguard.mullvad_endpoint(relay)
    }
}

#[derive(Default, Clone)]
pub struct WireguardMatcher {
    /// The peer is an already selected peer relay to be used with multihop.
    /// It's stored here so we can exclude it from further selections being made.
    pub peer: Option<Relay>,
    pub port: Constraint<u16>,
    pub ip_version: Constraint<IpVersion>,

    pub data: WireguardEndpointData,
}

impl WireguardMatcher {
    pub fn new(constraints: WireguardConstraints, data: WireguardEndpointData) -> Self {
        Self {
            peer: None,
            port: constraints.port,
            ip_version: constraints.ip_version,
            data,
        }
    }

    pub fn from_endpoint(data: WireguardEndpointData) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    fn wg_data_to_endpoint(
        &self,
        relay: &Relay,
        data: &WireguardEndpointData,
    ) -> Option<MullvadEndpoint> {
        let host = self.get_address_for_wireguard_relay(relay)?;
        let port = self.get_port_for_wireguard_relay(data)?;
        let peer_config = wireguard::PeerConfig {
            public_key: relay.endpoint_data.unwrap_wireguard_ref().public_key,
            endpoint: SocketAddr::new(host, port),
            allowed_ips: all_of_the_internet(),
            psk: None,
        };
        Some(MullvadEndpoint::Wireguard(MullvadWireguardEndpoint {
            peer: peer_config,
            exit_peer: None,
            ipv4_gateway: data.ipv4_gateway,
            ipv6_gateway: data.ipv6_gateway,
        }))
    }

    fn get_address_for_wireguard_relay(&self, relay: &Relay) -> Option<IpAddr> {
        match self.ip_version {
            Constraint::Any | Constraint::Only(IpVersion::V4) => Some(relay.ipv4_addr_in.into()),
            Constraint::Only(IpVersion::V6) => relay.ipv6_addr_in.map(|addr| addr.into()),
        }
    }

    fn get_port_for_wireguard_relay(&self, data: &WireguardEndpointData) -> Option<u16> {
        match self.port {
            Constraint::Any => {
                let get_port_amount =
                    |range: &(u16, u16)| -> u64 { (1 + range.1 - range.0) as u64 };
                let port_amount: u64 = data.port_ranges.iter().map(get_port_amount).sum();

                if port_amount < 1 {
                    return None;
                }

                let mut port_index = rand::thread_rng().gen_range(0, port_amount);

                for range in data.port_ranges.iter() {
                    let ports_in_range = get_port_amount(range);
                    if port_index < ports_in_range {
                        return Some(port_index as u16 + range.0);
                    }
                    port_index -= ports_in_range;
                }
                log::error!("Port selection algorithm is broken!");
                None
            }
            Constraint::Only(port) => {
                if data
                    .port_ranges
                    .iter()
                    .any(|range| (range.0 <= port && port <= range.1))
                {
                    Some(port)
                } else {
                    None
                }
            }
        }
    }
}

impl TunnelMatcher for WireguardMatcher {
    fn filter_matching_endpoints(&self, relay: &Relay) -> Option<Relay> {
        if self
            .peer
            .as_ref()
            .map(|peer_relay| peer_relay.hostname == relay.hostname)
            .unwrap_or(false)
        {
            return None;
        }
        if !matches!(relay.endpoint_data, RelayEndpointData::Wireguard(..)) {
            return None;
        }
        Some(relay.clone())
    }

    fn mullvad_endpoint(&self, relay: &Relay) -> Option<MullvadEndpoint> {
        self.wg_data_to_endpoint(relay, &self.data)
    }
}
