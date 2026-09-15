use std::sync::Arc;
use std::time::{Duration, Instant};

use fomoxa_net::schema::Schema;
use fomoxa_net::{Config, Connection, Event, SendError, TcpTransport};

use fomoxa_example_baseline::protocol::{self, ClientHello, PlayerInput, Welcome, WorldSnapshot};
use fomoxa_example_server::generated::{
    CLIENT_HELLO_GAME_MESSAGE_ID, PLAYER_INPUT_GAME_MESSAGE_ID, WELCOME_GAME_MESSAGE_ID,
    WORLD_SNAPSHOT_GAME_MESSAGE_ID,
};
use fomoxa_example_server::measure::{self, BotFactory, LoadBot, Options, Stats, Window, TAG_COUNT};
use fomoxa_example_server::schema::schema;
use fomoxa_example_server::client_kind;

const STACK: &str = "protobuf payloads over fomoxa-net";

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
                        if let Ok(welcome) = protocol::decode::<Welcome>(payload) {
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
                        let Ok(snapshot) = protocol::decode::<WorldSnapshot>(payload) else {
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
                client_kind: u32::from(client_kind::BOT),
                display_name: self.display_name.clone(),
            };
            let sent = self
                .connection
                .send(CLIENT_HELLO_GAME_MESSAGE_ID, &protocol::encode(&hello));
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
            .send(PLAYER_INPUT_GAME_MESSAGE_ID, &protocol::encode(&input))
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

fn main() {
    let options = Options::from_cli();
    let factory = Factory {
        address: options.address.clone(),
        schema: schema(),
    };
    let run = measure::run(STACK, &options, &factory);
    measure::report(STACK, &options, &run);
}
