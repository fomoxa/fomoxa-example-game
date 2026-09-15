use std::time::{Duration, Instant};

use fomoxa_example_baseline::framing::{self, Link, SendState};
use fomoxa_example_baseline::protocol::{self, ClientHello, PlayerInput, Welcome, WorldSnapshot};
use fomoxa_example_server::client_kind;
use fomoxa_example_server::measure::{self, BotFactory, LoadBot, Options, Stats, Window, TAG_COUNT};

const STACK: &str = "protobuf over plain TCP";
const DEFAULT_ADDRESS: &str = "127.0.0.1:9322";

struct Factory {
    address: String,
}

impl BotFactory for Factory {
    type Bot = Bot;

    fn connect(&self, index: usize, measured: bool, now: Instant) -> Result<Bot, String> {
        let mut link = Link::connect(self.address.as_str()).map_err(|error| error.to_string())?;
        let hello = ClientHello {
            client_kind: u32::from(client_kind::BOT),
            display_name: format!("bench-{index}"),
        };
        if link.send(framing::KIND_HELLO, &protocol::encode(&hello)) == SendState::Closed {
            return Err("the server closed the connection".to_owned());
        }
        Ok(Bot {
            link,
            started: now,
            phase: index as f32 * 0.37,
            player_id: None,
            sequence: 0,
            next_input: now,
            tag_cursor: 0,
            tag_sent: vec![None; if measured { TAG_COUNT } else { 0 }],
            measured,
            buffer: vec![0u8; framing::RECV_BUFFER],
            stats: Stats::measured(measured),
        })
    }
}

struct Bot {
    link: Link,
    started: Instant,
    phase: f32,
    player_id: Option<u32>,
    sequence: u32,
    next_input: Instant,
    tag_cursor: usize,
    tag_sent: Vec<Option<Instant>>,
    measured: bool,
    buffer: Vec<u8>,
    stats: Stats,
}

impl LoadBot for Bot {
    fn pump(&mut self, now: Instant, window: &Window, input_interval: Duration) {
        {
            let Bot {
                link,
                player_id,
                tag_sent,
                measured,
                buffer,
                stats,
                ..
            } = self;
            let measuring = window.measuring(now);
            link.poll(buffer, |kind, payload| match kind {
                framing::KIND_WELCOME => {
                    if let Ok(welcome) = protocol::decode::<Welcome>(payload) {
                        *player_id = Some(welcome.player_id);
                        stats.joined = true;
                    }
                }
                framing::KIND_SNAPSHOT => {
                    if measuring {
                        stats.snapshots += 1;
                        stats.snapshot_bytes += payload.len() as u64;
                    }
                    if !*measured {
                        return;
                    }
                    let Some(id) = *player_id else { return };
                    let Ok(snapshot) = protocol::decode::<WorldSnapshot>(payload) else {
                        return;
                    };
                    let Some(state) = snapshot.players.iter().find(|state| state.player_id == id) else {
                        return;
                    };
                    let Some(index) = measure::tag_index(state.look_pitch) else {
                        return;
                    };
                    let Some(sent) = tag_sent[index].take() else {
                        return;
                    };
                    if measuring && sent >= window.measure_start {
                        stats.rtt_micros.push(now.duration_since(sent).as_micros() as u32);
                    }
                }
                _ => {}
            });
            if link.is_closed() && stats.lost.is_none() {
                stats.lost = Some("the connection broke".to_owned());
            }
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

        match self.link.send(framing::KIND_INPUT, &protocol::encode(&input)) {
            SendState::Sent => {
                if self.measured {
                    self.tag_sent[self.tag_cursor] = Some(now);
                    self.tag_cursor = (self.tag_cursor + 1) % TAG_COUNT;
                }
                if measuring {
                    self.stats.inputs += 1;
                }
            }
            SendState::Congested => {
                if measuring {
                    self.stats.congested += 1;
                }
            }
            SendState::Closed => {
                if self.stats.lost.is_none() {
                    self.stats.lost = Some("the connection broke".to_owned());
                }
            }
        }
        self.next_input = now + input_interval;
    }

    fn into_stats(self) -> Stats {
        self.stats
    }
}

fn main() {
    let mut options = Options::from_cli();
    if options.address == measure::DEFAULT_ADDRESS {
        options.address = DEFAULT_ADDRESS.to_owned();
    }
    let factory = Factory {
        address: options.address.clone(),
    };
    let run = measure::run(STACK, &options, &factory);
    measure::report(STACK, &options, &run);
}
