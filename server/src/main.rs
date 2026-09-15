use std::collections::BTreeSet;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_net::{Config, Event, PeerId, SendError, Server, TcpListenerTransport};

use fomoxa_example_server::generated::{
    CLIENT_HELLO_GAME_MESSAGE_ID, PLAYER_INPUT_GAME_MESSAGE_ID, WELCOME_GAME_MESSAGE_ID,
    WORLD_SNAPSHOT_GAME_MESSAGE_ID,
};
use fomoxa_example_server::models::protocol::{ClientHello, PlayerInput, Welcome};
use fomoxa_example_server::peers::{describe_player, roster, PeerDirectory};
use fomoxa_example_server::schema::schema;
use fomoxa_example_server::world::{World, PLANE_HALF_SIZE, TICK_RATE};
use fomoxa_example_server::{cli, log_info, log_warn, messages};

const DEFAULT_ADDRESS: &str = "0.0.0.0:9321";
const DEFAULT_STATUS_INTERVAL_SECONDS: u64 = 30;
const IDLE_SLEEP: Duration = Duration::from_millis(2);

enum Inbound {
    Connected(PeerId),
    HandshakeAccepted(PeerId),
    HandshakeRefused(PeerId, String),
    Hello(PeerId, ClientHello),
    Input(PeerId, PlayerInput),
    Undecodable(PeerId, &'static str, String),
    Unexpected(PeerId, u32),
    Disconnected(PeerId, String),
}

struct Game {
    server: Server<TcpListenerTransport>,
    world: World,
    directory: PeerDirectory,
    pending_welcomes: BTreeSet<PeerId>,
    quiet: bool,
    snapshots_sent: u64,
    snapshots_dropped: u64,
    profile: Profile,
}

#[derive(Default)]
struct Profile {
    ticks: u32,
    loops: u32,
    events: Duration,
    step: Duration,
    encode: Duration,
    send: Duration,
}

impl Profile {
    fn line(&self, elapsed: Duration) -> String {
        let ticks = f64::from(self.ticks.max(1));
        let millis = |total: Duration| total.as_secs_f64() * 1000.0 / ticks;
        format!(
            "{:.1} Hz · {} loops/tick · per tick: events {:.2} ms · step {:.2} ms · encode {:.2} ms · send {:.2} ms",
            f64::from(self.ticks) / elapsed.as_secs_f64().max(f64::EPSILON),
            self.loops / self.ticks.max(1),
            millis(self.events),
            millis(self.step),
            millis(self.encode),
            millis(self.send)
        )
    }
}

fn main() {
    let address = cli::flag("--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let quiet = cli::has_flag("--quiet");
    let profiling = cli::has_flag("--profile");
    let status_interval = cli::flag("--status-interval")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_STATUS_INTERVAL_SECONDS);
    let listener = TcpListenerTransport::bind(address.as_str()).unwrap_or_else(|error| {
        log_warn!("cannot bind {address}: {error}");
        exit(1);
    });

    let mut game = Game {
        server: Server::new(listener, schema(), Config::default()),
        world: World::new(),
        directory: PeerDirectory::new(),
        pending_welcomes: BTreeSet::new(),
        quiet,
        snapshots_sent: 0,
        snapshots_dropped: 0,
        profile: Profile::default(),
    };
    let step = Duration::from_secs(1) / u32::from(TICK_RATE);
    let mut next_step = Instant::now() + step;
    let mut status_since = Instant::now();
    let mut next_status = (status_interval > 0).then(|| Instant::now() + Duration::from_secs(status_interval));

    log_info!(
        "listening   tcp {address} at {TICK_RATE} Hz, status every {}{}",
        if status_interval > 0 { format!("{status_interval}s") } else { "never".to_owned() },
        if quiet { ", per-peer logs off" } else { "" }
    );

    loop {
        let loop_started = Instant::now();
        for inbound in drain_events(&mut game.server) {
            game.handle(inbound);
        }

        game.flush_welcomes();

        let now = Instant::now();
        game.profile.events += now - loop_started;
        game.profile.loops += 1;
        if now >= next_step {
            game.world.step(step.as_secs_f32());
            let stepped = Instant::now();
            game.profile.step += stepped - now;
            game.broadcast_snapshot();
            game.profile.ticks += 1;
            next_step += step;
            if next_step <= now {
                next_step = now + step;
            }
        }

        if let Some(due) = next_status.filter(|due| now >= *due) {
            log_info!(
                "status      tick {} · {} connections · {} · snapshots {} sent, {} dropped",
                game.world.tick(),
                game.directory.len(),
                if game.quiet {
                    format!("online {}", game.world.player_count())
                } else {
                    roster(&game.world, &game.directory)
                },
                game.snapshots_sent,
                game.snapshots_dropped
            );
            if profiling {
                log_info!("profile     {}", game.profile.line(now - status_since));
            }
            game.profile = Profile::default();
            status_since = now;
            next_status = Some(due + Duration::from_secs(status_interval));
        }

        thread::sleep(IDLE_SLEEP);
    }
}

fn drain_events(server: &mut Server<TcpListenerTransport>) -> Vec<Inbound> {
    let mut inbound = Vec::new();
    for seen in server.tick_now() {
        let peer = seen.peer;
        inbound.push(match seen.event {
            Event::Connected => Inbound::Connected(peer),
            Event::Ready => Inbound::HandshakeAccepted(peer),
            Event::HandshakeFailed(reason) => Inbound::HandshakeRefused(peer, reason.to_string()),
            Event::Disconnected(reason) => Inbound::Disconnected(peer, reason.to_string()),
            Event::Message { id: CLIENT_HELLO_GAME_MESSAGE_ID, payload } => {
                match messages::decode_client_hello(payload) {
                    Ok(hello) => Inbound::Hello(peer, hello),
                    Err(error) => Inbound::Undecodable(peer, "ClientHello", error.to_string()),
                }
            }
            Event::Message { id: PLAYER_INPUT_GAME_MESSAGE_ID, payload } => {
                match messages::decode_player_input(payload) {
                    Ok(input) => Inbound::Input(peer, input),
                    Err(error) => Inbound::Undecodable(peer, "PlayerInput", error.to_string()),
                }
            }
            Event::Message { id, .. } => Inbound::Unexpected(peer, id),
            Event::Probe | Event::Ack => continue,
        });
    }
    inbound
}

impl Game {
    fn handle(&mut self, inbound: Inbound) {
        let now = Instant::now();
        match inbound {
            Inbound::Connected(peer) => {
                let address = self
                    .server
                    .transport(peer)
                    .and_then(|transport| transport.peer_addr().ok());
                self.directory.connect(peer, address, now);
                if !self.quiet {
                    log_info!(
                        "connect     {peer} from {} · {} connections",
                        self.directory.address_label(peer),
                        self.directory.len()
                    );
                }
            }
            Inbound::HandshakeAccepted(peer) => {
                self.directory.accept_handshake(peer);
                if !self.quiet {
                    log_info!("handshake   {peer} from {} accepted", self.directory.address_label(peer));
                }
            }
            Inbound::HandshakeRefused(peer, reason) => {
                log_warn!(
                    "handshake   {peer} from {} refused: {reason} (schema fingerprint mismatch? run tools/check-fingerprints.sh)",
                    self.directory.address_label(peer)
                );
                self.forget(peer, now, &reason);
            }
            Inbound::Hello(peer, hello) => {
                if self.world.player(peer).is_some() {
                    log_warn!("join        {peer} sent ClientHello again, ignored");
                    return;
                }
                self.world.join(peer, hello.client_kind, hello.display_name);
                self.pending_welcomes.insert(peer);
                if self.quiet {
                    return;
                }
                if let Some(player) = self.world.player(peer) {
                    log_info!(
                        "join        {} from {} ({peer})",
                        describe_player(player),
                        self.directory.address_label(peer)
                    );
                }
                log_info!("roster      {}", roster(&self.world, &self.directory));
            }
            Inbound::Input(peer, input) => {
                self.world.apply_input(peer, &input);
            }
            Inbound::Undecodable(peer, message, error) => {
                log_warn!(
                    "decode      {peer} from {} sent an undecodable {message}: {error}",
                    self.directory.address_label(peer)
                );
            }
            Inbound::Unexpected(peer, id) => {
                log_warn!(
                    "message     {peer} from {} sent unexpected message id 0x{id:08X}",
                    self.directory.address_label(peer)
                );
            }
            Inbound::Disconnected(peer, reason) => self.forget(peer, now, &reason),
        }
    }

    fn forget(&mut self, peer: PeerId, now: Instant, reason: &str) {
        self.pending_welcomes.remove(&peer);
        let record = self.directory.remove(peer);
        let address = record.map_or_else(|| "unknown address".to_owned(), |record| record.address_label());
        let connected_for = record.map_or(0.0, |record| record.connected_for(now).as_secs_f32());

        let left = self.world.leave(peer);
        if self.quiet {
            return;
        }

        match left {
            Some(player) => {
                log_info!(
                    "leave       {} from {address} ({peer}) after {connected_for:.1}s: {reason}",
                    describe_player(&player)
                );
                log_info!("roster      {}", roster(&self.world, &self.directory));
            }
            None => {
                let stage = match record {
                    Some(record) if record.handshake_accepted => "before sending ClientHello",
                    Some(_) => "during handshake",
                    None => "before connecting",
                };
                log_info!(
                    "disconnect  {peer} from {address} after {connected_for:.1}s {stage}: {reason}"
                );
            }
        }
    }

    fn flush_welcomes(&mut self) {
        let Game {
            server,
            world,
            pending_welcomes,
            ..
        } = self;
        pending_welcomes.retain(|&peer| {
            let Some(player) = world.player(peer) else {
                return false;
            };
            let welcome = Welcome {
                player_id: player.id,
                plane_half_size: PLANE_HALF_SIZE,
                tick_rate: TICK_RATE,
            };
            match server.send(peer, WELCOME_GAME_MESSAGE_ID, &messages::encode_welcome(&welcome)) {
                Ok(()) => false,
                Err(SendError::Congested) => true,
                Err(error) => {
                    log_warn!("welcome     {peer} could not be welcomed: {error}");
                    false
                }
            }
        });
    }

    fn broadcast_snapshot(&mut self) {
        let Game {
            server,
            world,
            snapshots_sent,
            snapshots_dropped,
            profile,
            ..
        } = self;
        let encode_started = Instant::now();
        let payload = messages::encode_world_snapshot(&world.snapshot());
        let send_started = Instant::now();
        profile.encode += send_started - encode_started;
        for peer in world.peers() {
            match server.send(peer, WORLD_SNAPSHOT_GAME_MESSAGE_ID, &payload) {
                Ok(()) => *snapshots_sent += 1,
                Err(_) => *snapshots_dropped += 1,
            }
        }
        profile.send += send_started.elapsed();
    }
}
