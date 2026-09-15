use std::collections::BTreeSet;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_net::{Config, Event, PeerId, SendError, Server, TcpListenerTransport};

use fomoxa_example_baseline::protocol::{self, ClientHello, PlayerInput, Welcome};
use fomoxa_example_server::generated::{
    CLIENT_HELLO_GAME_MESSAGE_ID, PLAYER_INPUT_GAME_MESSAGE_ID, WELCOME_GAME_MESSAGE_ID,
    WORLD_SNAPSHOT_GAME_MESSAGE_ID,
};
use fomoxa_example_server::schema::schema;
use fomoxa_example_server::world::{World, PLANE_HALF_SIZE, TICK_RATE};
use fomoxa_example_server::{cli, log_info, log_warn};

const DEFAULT_ADDRESS: &str = "0.0.0.0:9321";
const IDLE_SLEEP: Duration = Duration::from_millis(2);
const STACK: &str = "protobuf payloads over fomoxa-net";

enum Inbound {
    Hello(PeerId, ClientHello),
    Input(PeerId, PlayerInput),
    Gone(PeerId),
}

fn main() {
    let address = cli::flag("--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let profiling = cli::has_flag("--profile");
    let status_interval = cli::flag("--status-interval")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let listener = TcpListenerTransport::bind(address.as_str()).unwrap_or_else(|error| {
        log_warn!("cannot bind {address}: {error}");
        exit(1);
    });

    let mut server = Server::new(listener, schema(), Config::default());
    let mut world = World::new();
    let mut pending_welcomes: BTreeSet<PeerId> = BTreeSet::new();
    let mut sent = 0u64;
    let mut dropped = 0u64;
    let (mut ticks, mut loops) = (0u32, 0u32);
    let (mut events_total, mut step_total, mut encode_total, mut send_total) = (
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
    );

    let step = Duration::from_secs(1) / u32::from(TICK_RATE);
    let mut next_step = Instant::now() + step;
    let mut status_since = Instant::now();
    let mut next_status = (status_interval > 0).then(|| Instant::now() + Duration::from_secs(status_interval));

    log_info!("listening   tcp {address} at {TICK_RATE} Hz · {STACK}");

    loop {
        let loop_started = Instant::now();
        let mut inbound = Vec::new();
        for seen in server.tick_now() {
            let peer = seen.peer;
            match seen.event {
                Event::Message {
                    id: CLIENT_HELLO_GAME_MESSAGE_ID,
                    payload,
                } => match protocol::decode::<ClientHello>(payload) {
                    Ok(hello) => inbound.push(Inbound::Hello(peer, hello)),
                    Err(error) => log_warn!("decode      {peer} sent an undecodable ClientHello: {error}"),
                },
                Event::Message {
                    id: PLAYER_INPUT_GAME_MESSAGE_ID,
                    payload,
                } => match protocol::decode::<PlayerInput>(payload) {
                    Ok(input) => inbound.push(Inbound::Input(peer, input)),
                    Err(error) => log_warn!("decode      {peer} sent an undecodable PlayerInput: {error}"),
                },
                Event::Disconnected(_) | Event::HandshakeFailed(_) => inbound.push(Inbound::Gone(peer)),
                _ => {}
            }
        }

        for message in inbound {
            match message {
                Inbound::Hello(peer, hello) => {
                    if world.player(peer).is_none() {
                        world.join(peer, hello.client_kind as u8, hello.display_name);
                        pending_welcomes.insert(peer);
                    }
                }
                Inbound::Input(peer, input) => {
                    world.apply_input(peer, &input.as_fomoxa());
                }
                Inbound::Gone(peer) => {
                    pending_welcomes.remove(&peer);
                    world.leave(peer);
                }
            }
        }

        pending_welcomes.retain(|&peer| {
            let Some(player) = world.player(peer) else {
                return false;
            };
            let welcome = Welcome {
                player_id: player.id,
                plane_half_size: PLANE_HALF_SIZE,
                tick_rate: u32::from(TICK_RATE),
            };
            match server.send(peer, WELCOME_GAME_MESSAGE_ID, &protocol::encode(&welcome)) {
                Ok(()) => false,
                Err(SendError::Congested) => true,
                Err(_) => false,
            }
        });

        let now = Instant::now();
        events_total += now - loop_started;
        loops += 1;
        if now >= next_step {
            world.step(step.as_secs_f32());
            let stepped = Instant::now();
            step_total += stepped - now;

            let payload = protocol::encode(&protocol::snapshot_of(&world));
            let encoded = Instant::now();
            encode_total += encoded - stepped;

            for peer in world.peers() {
                match server.send(peer, WORLD_SNAPSHOT_GAME_MESSAGE_ID, &payload) {
                    Ok(()) => sent += 1,
                    Err(_) => dropped += 1,
                }
            }
            send_total += encoded.elapsed();
            ticks += 1;

            next_step += step;
            if next_step <= now {
                next_step = now + step;
            }
        }

        if let Some(due) = next_status.filter(|due| now >= *due) {
            log_info!(
                "status      tick {} · online {} · snapshots {sent} sent, {dropped} dropped",
                world.tick(),
                world.player_count()
            );
            if profiling {
                let per_tick = |total: Duration| total.as_secs_f64() * 1000.0 / f64::from(ticks.max(1));
                log_info!(
                    "profile     {:.1} Hz · {} loops/tick · per tick: events {:.2} ms · step {:.2} ms · encode {:.2} ms · send {:.2} ms",
                    f64::from(ticks) / (now - status_since).as_secs_f64().max(f64::EPSILON),
                    loops / ticks.max(1),
                    per_tick(events_total),
                    per_tick(step_total),
                    per_tick(encode_total),
                    per_tick(send_total)
                );
            }
            ticks = 0;
            loops = 0;
            events_total = Duration::ZERO;
            step_total = Duration::ZERO;
            encode_total = Duration::ZERO;
            send_total = Duration::ZERO;
            status_since = now;
            next_status = Some(due + Duration::from_secs(status_interval));
        }

        thread::sleep(IDLE_SLEEP);
    }
}
