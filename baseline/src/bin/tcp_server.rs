use std::io::ErrorKind;
use std::net::TcpListener;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_net::PeerId;

use fomoxa_example_baseline::framing::{self, Link, SendState};
use fomoxa_example_baseline::protocol::{self, ClientHello, PlayerInput, Welcome};
use fomoxa_example_server::world::{World, PLANE_HALF_SIZE, TICK_RATE};
use fomoxa_example_server::{cli, log_info, log_warn};

const DEFAULT_ADDRESS: &str = "0.0.0.0:9322";
const IDLE_SLEEP: Duration = Duration::from_millis(2);
const STACK: &str = "protobuf over plain TCP";

enum Action {
    Hello(ClientHello),
    Input(PlayerInput),
}

struct Peer {
    link: Link,
    id: PeerId,
    welcome_pending: bool,
}

fn main() {
    let address = cli::flag("--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let profiling = cli::has_flag("--profile");
    let status_interval = cli::flag("--status-interval")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let listener = TcpListener::bind(address.as_str()).unwrap_or_else(|error| {
        log_warn!("cannot bind {address}: {error}");
        exit(1);
    });
    listener.set_nonblocking(true).unwrap_or_else(|error| {
        log_warn!("cannot set {address} non-blocking: {error}");
        exit(1);
    });

    let mut world = World::new();
    let mut peers: Vec<Peer> = Vec::new();
    let mut next_peer = 1u64;
    let mut buffer = vec![0u8; framing::RECV_BUFFER];
    let mut actions: Vec<(usize, Action)> = Vec::new();
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

        loop {
            match listener.accept() {
                Ok((stream, _)) => match Link::adopt(stream) {
                    Ok(link) => {
                        peers.push(Peer {
                            link,
                            id: PeerId(next_peer),
                            welcome_pending: false,
                        });
                        next_peer += 1;
                    }
                    Err(error) => log_warn!("accept      cannot adopt a connection: {error}"),
                },
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => {
                    log_warn!("accept      {error}");
                    break;
                }
            }
        }

        actions.clear();
        for (index, peer) in peers.iter_mut().enumerate() {
            peer.link.poll(&mut buffer, |kind, payload| match kind {
                framing::KIND_HELLO => match protocol::decode::<ClientHello>(payload) {
                    Ok(hello) => actions.push((index, Action::Hello(hello))),
                    Err(error) => log_warn!("decode      undecodable ClientHello: {error}"),
                },
                framing::KIND_INPUT => match protocol::decode::<PlayerInput>(payload) {
                    Ok(input) => actions.push((index, Action::Input(input))),
                    Err(error) => log_warn!("decode      undecodable PlayerInput: {error}"),
                },
                _ => log_warn!("message     unexpected kind {kind}"),
            });
        }

        for (index, action) in actions.drain(..) {
            let peer = &mut peers[index];
            match action {
                Action::Hello(hello) => {
                    if world.player(peer.id).is_none() {
                        world.join(peer.id, hello.client_kind as u8, hello.display_name);
                        peer.welcome_pending = true;
                    }
                }
                Action::Input(input) => {
                    world.apply_input(peer.id, &input.as_fomoxa());
                }
            }
        }

        for peer in peers.iter_mut().filter(|peer| peer.welcome_pending) {
            let Some(player) = world.player(peer.id) else {
                peer.welcome_pending = false;
                continue;
            };
            let welcome = Welcome {
                player_id: player.id,
                plane_half_size: PLANE_HALF_SIZE,
                tick_rate: u32::from(TICK_RATE),
            };
            if peer.link.send(framing::KIND_WELCOME, &protocol::encode(&welcome)) != SendState::Congested {
                peer.welcome_pending = false;
            }
        }

        peers.retain(|peer| {
            if peer.link.is_closed() {
                world.leave(peer.id);
                false
            } else {
                true
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

            for peer in peers.iter_mut() {
                if world.player(peer.id).is_none() {
                    continue;
                }
                match peer.link.send(framing::KIND_SNAPSHOT, &payload) {
                    SendState::Sent => sent += 1,
                    _ => dropped += 1,
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
