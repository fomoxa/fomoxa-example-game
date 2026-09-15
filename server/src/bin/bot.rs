use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

use fomoxa_net::{Config, Connection, Event, SendError, TcpTransport};

use fomoxa_example_server::generated::{
    CLIENT_HELLO_GAME_MESSAGE_ID, PLAYER_INPUT_GAME_MESSAGE_ID, WELCOME_GAME_MESSAGE_ID,
    WORLD_SNAPSHOT_GAME_MESSAGE_ID,
};
use fomoxa_example_server::models::protocol::{ClientHello, PlayerInput, WorldSnapshot};
use fomoxa_example_server::schema::schema;
use fomoxa_example_server::{cli, client_kind, messages};

const DEFAULT_ADDRESS: &str = "127.0.0.1:9321";
const INPUT_INTERVAL: Duration = Duration::from_millis(33);
const REPORT_INTERVAL: Duration = Duration::from_secs(1);
const IDLE_SLEEP: Duration = Duration::from_millis(5);

#[derive(Default)]
struct Observed {
    player_id: Option<u32>,
    latest: Option<WorldSnapshot>,
    most_players: usize,
    saw_itself: bool,
}

impl Observed {
    fn record(&mut self, snapshot: WorldSnapshot) {
        self.most_players = self.most_players.max(snapshot.players.len());
        if let Some(player_id) = self.player_id {
            self.saw_itself |= snapshot.players.iter().any(|state| state.player_id == player_id);
        }
        self.latest = Some(snapshot);
    }

    fn summary(&self, display_name: &str) -> String {
        let Some(snapshot) = &self.latest else {
            return format!("{display_name}: no snapshot yet");
        };
        let players = snapshot
            .players
            .iter()
            .map(|state| {
                format!(
                    "#{} {} pos ({:.1}, {:.1}, {:.1}) look ({:.2}, {:.2})",
                    state.player_id,
                    client_kind::name(state.client_kind),
                    state.position_x,
                    state.position_y,
                    state.position_z,
                    state.look_yaw,
                    state.look_pitch
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{display_name} #{}: tick {} · {} players: {players}",
            self.player_id.unwrap_or_default(),
            snapshot.tick,
            snapshot.players.len()
        )
    }
}

fn main() {
    let address = cli::flag("--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned());
    let display_name = cli::flag("--name").unwrap_or_else(|| "bot".to_owned());
    let run_for = cli::flag("--seconds")
        .and_then(|value| value.parse::<f32>().ok())
        .map(Duration::from_secs_f32);
    let expected_players = cli::flag("--expect-players")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);

    let transport = TcpTransport::connect(address.as_str()).unwrap_or_else(|error| {
        eprintln!("{display_name}: cannot reach {address}: {error}");
        exit(1);
    });
    let mut connection = Connection::new(transport, schema(), Config::default());
    let started = Instant::now();
    let mut observed = Observed::default();
    let mut hello_pending = false;
    let mut sequence = 0u32;
    let mut last_input = started;
    let mut last_report = started;

    loop {
        let mut failure = None;
        for event in connection.tick_now() {
            match event {
                Event::Ready => hello_pending = true,
                Event::Message { id: WELCOME_GAME_MESSAGE_ID, payload } => {
                    match messages::decode_welcome(payload) {
                        Ok(welcome) => {
                            observed.player_id = Some(welcome.player_id);
                            println!(
                                "{display_name}: joined as player {} on a {}x{} plane at {} Hz",
                                welcome.player_id,
                                welcome.plane_half_size * 2.0,
                                welcome.plane_half_size * 2.0,
                                welcome.tick_rate
                            );
                        }
                        Err(error) => eprintln!("{display_name}: undecodable Welcome: {error}"),
                    }
                }
                Event::Message { id: WORLD_SNAPSHOT_GAME_MESSAGE_ID, payload } => {
                    match messages::decode_world_snapshot(payload) {
                        Ok(snapshot) => observed.record(snapshot),
                        Err(error) => eprintln!("{display_name}: undecodable WorldSnapshot: {error}"),
                    }
                }
                Event::HandshakeFailed(reason) => failure = Some(format!("handshake refused: {reason}")),
                Event::Disconnected(reason) => failure = Some(format!("disconnected: {reason}")),
                _ => {}
            }
        }

        if let Some(reason) = failure {
            eprintln!("{display_name}: {reason}");
            exit(1);
        }

        if hello_pending {
            let hello = ClientHello {
                client_kind: client_kind::BOT,
                display_name: display_name.clone(),
            };
            let sent = connection.send(CLIENT_HELLO_GAME_MESSAGE_ID, &messages::encode_client_hello(&hello));
            hello_pending = matches!(sent, Err(SendError::Congested));
        }

        if observed.player_id.is_some() && last_input.elapsed() >= INPUT_INTERVAL {
            let angle = started.elapsed().as_secs_f32();
            sequence = sequence.wrapping_add(1);
            let (move_x, move_z) = (angle.cos(), angle.sin());
            let input = PlayerInput {
                sequence,
                move_x,
                move_z,
                jump: true,
                look_yaw: (-move_x).atan2(-move_z),
                look_pitch: 0.4 * (angle * 2.0).sin(),
            };
            if connection
                .send(PLAYER_INPUT_GAME_MESSAGE_ID, &messages::encode_player_input(&input))
                .is_ok()
            {
                last_input = Instant::now();
            }
        }

        if last_report.elapsed() >= REPORT_INTERVAL {
            println!("{}", observed.summary(&display_name));
            last_report = Instant::now();
        }

        if run_for.is_some_and(|limit| started.elapsed() >= limit) {
            let passed = observed.saw_itself && observed.most_players >= expected_players;
            println!(
                "{display_name}: {} (saw itself: {}, most players: {}, expected at least {expected_players})",
                if passed { "PASS" } else { "FAIL" },
                observed.saw_itself,
                observed.most_players
            );
            exit(if passed { 0 } else { 1 });
        }

        thread::sleep(IDLE_SLEEP);
    }
}
