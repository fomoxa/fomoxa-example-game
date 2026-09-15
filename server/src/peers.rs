use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use fomoxa_net::PeerId;

use crate::client_kind;
use crate::world::{Player, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerRecord {
    pub address: Option<SocketAddr>,
    pub connected_at: Instant,
    pub handshake_accepted: bool,
}

impl PeerRecord {
    pub fn address_label(&self) -> String {
        address_label(self.address)
    }

    pub fn connected_for(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.connected_at)
    }
}

#[derive(Debug, Default)]
pub struct PeerDirectory {
    records: BTreeMap<PeerId, PeerRecord>,
}

impl PeerDirectory {
    pub fn new() -> PeerDirectory {
        PeerDirectory::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn connect(&mut self, peer: PeerId, address: Option<SocketAddr>, now: Instant) {
        self.records.insert(
            peer,
            PeerRecord {
                address,
                connected_at: now,
                handshake_accepted: false,
            },
        );
    }

    pub fn accept_handshake(&mut self, peer: PeerId) {
        if let Some(record) = self.records.get_mut(&peer) {
            record.handshake_accepted = true;
        }
    }

    pub fn get(&self, peer: PeerId) -> Option<&PeerRecord> {
        self.records.get(&peer)
    }

    pub fn address_label(&self, peer: PeerId) -> String {
        address_label(self.records.get(&peer).and_then(|record| record.address))
    }

    pub fn remove(&mut self, peer: PeerId) -> Option<PeerRecord> {
        self.records.remove(&peer)
    }
}

pub fn address_label(address: Option<SocketAddr>) -> String {
    address.map_or_else(|| "unknown address".to_owned(), |address| address.to_string())
}

pub fn describe_player(player: &Player) -> String {
    format!(
        "#{} {} \"{}\"",
        player.id,
        client_kind::name(player.client_kind),
        player.display_name
    )
}

pub fn roster(world: &World, directory: &PeerDirectory) -> String {
    let entries = world
        .peers()
        .filter_map(|peer| world.player(peer).map(|player| (peer, player)))
        .map(|(peer, player)| {
            format!(
                "{} @{} ({peer})",
                describe_player(player),
                directory.address_label(peer)
            )
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "online 0".to_owned()
    } else {
        format!("online {}: {}", entries.len(), entries.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEER: PeerId = PeerId(7);

    fn loopback(port: u16) -> Option<SocketAddr> {
        Some(SocketAddr::from(([127, 0, 0, 1], port)))
    }

    #[test]
    fn a_connected_peer_reports_its_address_until_removed() {
        let mut directory = PeerDirectory::new();
        directory.connect(PEER, loopback(50_000), Instant::now());

        assert_eq!(directory.address_label(PEER), "127.0.0.1:50000");

        let removed = directory.remove(PEER).unwrap();
        assert_eq!(removed.address_label(), "127.0.0.1:50000");
        assert_eq!(directory.address_label(PEER), "unknown address");
        assert!(directory.is_empty());
    }

    #[test]
    fn accepting_the_handshake_is_recorded() {
        let mut directory = PeerDirectory::new();
        directory.connect(PEER, None, Instant::now());

        directory.accept_handshake(PEER);

        assert!(directory.get(PEER).unwrap().handshake_accepted);
    }

    #[test]
    fn connection_time_is_measured_from_connect() {
        let connected_at = Instant::now();
        let mut directory = PeerDirectory::new();
        directory.connect(PEER, None, connected_at);

        let record = directory.get(PEER).unwrap();

        assert_eq!(record.connected_for(connected_at + Duration::from_millis(1_500)), Duration::from_millis(1_500));
    }

    #[test]
    fn roster_lists_every_joined_player_with_its_address() {
        let mut world = World::new();
        let mut directory = PeerDirectory::new();
        directory.connect(PEER, loopback(40_000), Instant::now());
        world.join(PEER, client_kind::UNITY, "bob".to_owned());

        assert_eq!(
            roster(&world, &directory),
            "online 1: #1 unity \"bob\" @127.0.0.1:40000 (peer#7)"
        );
    }

    #[test]
    fn an_empty_world_has_an_empty_roster() {
        assert_eq!(roster(&World::new(), &PeerDirectory::new()), "online 0");
    }
}
