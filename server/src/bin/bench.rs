use std::sync::Arc;
use std::time::{Duration, Instant};

use fomoxa_net::schema::Schema;
use fomoxa_net::{Config, Connection, Event, SendError, TcpTransport};

use fomoxa_example_server::generated::{
    CLIENT_HELLO_GAME_MESSAGE_ID, PLAYER_INPUT_GAME_MESSAGE_ID, WELCOME_GAME_MESSAGE_ID,
    WORLD_SNAPSHOT_GAME_MESSAGE_ID,
};
use fomoxa_example_server::measure::{
    self, BotFactory, LoadBot, Options, Stats, Window, TAG_COUNT,
};
use fomoxa_example_server::models::protocol::{ClientHello, PlayerInput, PlayerState, WorldSnapshot};
use fomoxa_example_server::schema::schema;
use fomoxa_example_server::{cli, client_kind, messages};

const STACK: &str = "fomoxa (fomoxa-net + fomoxac codec)";

struct Factory {
    address: String,
    schema: Arc<Schema>,
}

impl BotFactory for Factory {
    type Bot = Bot;

    fn connect(&self, index: usize, measured: bool, now: Instant) -> Result<Bot, String> {
        let transport = TcpTransport::connect(self.address.as_str()).map_err(|error| error.to_string())?;
        Ok(Bot {
            connection: Connection::new(transport, Arc::clone(&self.schema), Config::default()),
            display_name: format!("bench-{index}"),
            started: now,
            phase: index as f32 * 0.37,
            player_id: None,
            hello_pending: false,
            sequence: 0,
            next_input: now,
            tag_cursor: 0,
            tag_sent: vec![None; if measured { TAG_COUNT } else { 0 }],
            measured,
            stats: Stats::measured(measured),
        })
    }
}

struct Bot {
    connection: Connection<TcpTransport>,
    display_name: String,
    started: Instant,
    phase: f32,
    player_id: Option<u32>,
    hello_pending: bool,
    sequence: u32,
    next_input: Instant,
    tag_cursor: usize,
    tag_sent: Vec<Option<Instant>>,
    measured: bool,
    stats: Stats,
}

impl LoadBot for Bot {
    fn pump(&mut self, now: Instant, window: &Window, input_interval: Duration) {
        {
            let Bot {
                connection,
                player_id,
                hello_pending,
                tag_sent,
                measured,
                stats,
                ..
            } = self;
            let measuring = window.measuring(now);
            for event in connection.tick_now() {
                match event {
                    Event::Ready => *hello_pending = true,
                    Event::Message {
                        id: WELCOME_GAME_MESSAGE_ID,
                        payload,
                    } => {
                        if let Ok(welcome) = messages::decode_welcome(payload) {
                            *player_id = Some(welcome.player_id);
                            stats.joined = true;
                        }
                    }
                    Event::Message {
                        id: WORLD_SNAPSHOT_GAME_MESSAGE_ID,
                        payload,
                    } => {
                        if measuring {
                            stats.snapshots += 1;
                            stats.snapshot_bytes += payload.len() as u64;
                        }
                        if !*measured {
                            continue;
                        }
                        let Some(id) = *player_id else { continue };
                        let Ok(snapshot) = messages::decode_world_snapshot(payload) else {
                            continue;
                        };
                        let Some(state) = snapshot.players.iter().find(|state| state.player_id == id) else {
                            continue;
                        };
                        let Some(index) = measure::tag_index(state.look_pitch) else {
                            continue;
                        };
                        let Some(sent) = tag_sent[index].take() else {
                            continue;
                        };
                        if measuring && sent >= window.measure_start {
                            stats.rtt_micros.push(now.duration_since(sent).as_micros() as u32);
                        }
                    }
                    Event::HandshakeFailed(reason) => stats.lost = Some(reason.to_string()),
                    Event::Disconnected(reason) => stats.lost = Some(reason.to_string()),
                    _ => {}
                }
            }
        }

        if self.hello_pending {
            let hello = ClientHello {
                client_kind: client_kind::BOT,
                display_name: self.display_name.clone(),
            };
            let sent = self
                .connection
                .send(CLIENT_HELLO_GAME_MESSAGE_ID, &messages::encode_client_hello(&hello));
            self.hello_pending = matches!(sent, Err(SendError::Congested));
        }

        if self.player_id.is_none() || now < self.next_input {
            return;
        }

        let measuring = window.measuring(now);
        let (move_x, move_z, look_yaw) =
            measure::circling_input(self.started.elapsed().as_secs_f32(), self.phase);
        let look_pitch = if self.measured {
            measure::tag_pitch(self.tag_cursor)
        } else {
            0.0
        };
        self.sequence = self.sequence.wrapping_add(1);
        let input = PlayerInput {
            sequence: self.sequence,
            move_x,
            move_z,
            jump: true,
            look_yaw,
            look_pitch,
        };

        match self
            .connection
            .send(PLAYER_INPUT_GAME_MESSAGE_ID, &messages::encode_player_input(&input))
        {
            Ok(()) => {
                if self.measured {
                    self.tag_sent[self.tag_cursor] = Some(now);
                    self.tag_cursor = (self.tag_cursor + 1) % TAG_COUNT;
                }
                if measuring {
                    self.stats.inputs += 1;
                }
            }
            Err(SendError::Congested) => {
                if measuring {
                    self.stats.congested += 1;
                }
            }
            Err(_) => {}
        }
        self.next_input = now + input_interval;
    }

    fn into_stats(self) -> Stats {
        self.stats
    }
}

fn codec_bench(players: usize, rounds: usize) {
    let snapshot = WorldSnapshot {
        tick: 1,
        players: (0..players)
            .map(|index| PlayerState {
                player_id: index as u32,
                client_kind: client_kind::BOT,
                color: 0xE6194BFF,
                position_x: index as f32 * 0.01,
                position_z: index as f32 * -0.01,
                position_y: 0.5,
                look_yaw: 0.25,
                look_pitch: -0.25,
            })
            .collect(),
    };

    let encode_started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..rounds {
        bytes += messages::encode_world_snapshot(&snapshot).len();
    }
    let encode_elapsed = encode_started.elapsed();

    let payload = messages::encode_world_snapshot(&snapshot);
    let decode_started = Instant::now();
    let mut decoded = 0usize;
    for _ in 0..rounds {
        decoded += messages::decode_world_snapshot(&payload)
            .map(|snapshot| snapshot.players.len())
            .unwrap_or(0);
    }
    let decode_elapsed = decode_started.elapsed();

    let per_round = |elapsed: Duration| elapsed.as_secs_f64() * 1e6 / rounds as f64;
    let throughput = |elapsed: Duration| {
        payload.len() as f64 * rounds as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0 * 1024.0)
    };

    println!("codec: fomoxac · {players} players, {} bytes, {rounds} rounds", payload.len());
    println!(
        "encode          {:.1} us per snapshot · {:.1} ns per player · {:.2} GiB/s",
        per_round(encode_elapsed),
        encode_elapsed.as_secs_f64() * 1e9 / (rounds * players) as f64,
        throughput(encode_elapsed)
    );
    println!(
        "decode          {:.1} us per snapshot · {:.1} ns per player · {:.2} GiB/s",
        per_round(decode_elapsed),
        decode_elapsed.as_secs_f64() * 1e9 / (rounds * players) as f64,
        throughput(decode_elapsed)
    );
    println!("checksum        {bytes} bytes encoded, {decoded} players decoded");
}

fn main() {
    if let Some(players) = cli::flag("--codec").and_then(|value| value.parse::<usize>().ok()) {
        let rounds = cli::flag("--rounds")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10_000);
        codec_bench(players.max(1), rounds.max(1));
        return;
    }

    let options = Options::from_cli();
    let factory = Factory {
        address: options.address.clone(),
        schema: schema(),
    };
    let run = measure::run(STACK, &options, &factory);
    measure::report(STACK, &options, &run);
}
