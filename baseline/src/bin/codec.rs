use std::time::{Duration, Instant};

use fomoxa_example_baseline::protocol::{PlayerState, WorldSnapshot};
use fomoxa_example_server::cli;
use fomoxa_example_server::client_kind;
use prost::Message;

fn snapshot(players: usize) -> WorldSnapshot {
    WorldSnapshot {
        tick: 1,
        players: (0..players)
            .map(|index| PlayerState {
                player_id: index as u32,
                client_kind: u32::from(client_kind::BOT),
                color: 0xE6194BFF,
                position_x: index as f32 * 0.01,
                position_z: index as f32 * -0.01,
                position_y: 0.5,
                look_yaw: 0.25,
                look_pitch: -0.25,
            })
            .collect(),
    }
}

fn main() {
    let players = cli::flag("--players")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000)
        .max(1);
    let rounds = cli::flag("--rounds")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000)
        .max(1);

    let snapshot = snapshot(players);
    let payload = snapshot.encode_to_vec();

    let encode_started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..rounds {
        bytes += snapshot.encode_to_vec().len();
    }
    let encode_elapsed = encode_started.elapsed();

    let decode_started = Instant::now();
    let mut decoded = 0usize;
    for _ in 0..rounds {
        decoded += WorldSnapshot::decode(&payload[..])
            .map(|snapshot| snapshot.players.len())
            .unwrap_or(0);
    }
    let decode_elapsed = decode_started.elapsed();

    let per_round = |elapsed: Duration| elapsed.as_secs_f64() * 1e6 / rounds as f64;
    let per_player = |elapsed: Duration| elapsed.as_secs_f64() * 1e9 / (rounds * players) as f64;
    let throughput = |elapsed: Duration| {
        payload.len() as f64 * rounds as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0 * 1024.0)
    };

    println!(
        "codec: prost (protobuf) · {players} players, {} bytes, {rounds} rounds",
        payload.len()
    );
    println!(
        "encode          {:.1} us per snapshot · {:.1} ns per player · {:.2} GiB/s",
        per_round(encode_elapsed),
        per_player(encode_elapsed),
        throughput(encode_elapsed)
    );
    println!(
        "decode          {:.1} us per snapshot · {:.1} ns per player · {:.2} GiB/s",
        per_round(decode_elapsed),
        per_player(decode_elapsed),
        throughput(decode_elapsed)
    );
    println!(
        "bytes           {:.1} per player (snapshot header {} bytes)",
        (payload.len() - snapshot_header(&snapshot)) as f64 / players as f64,
        snapshot_header(&snapshot)
    );
    println!("checksum        {bytes} bytes encoded, {decoded} players decoded");
}

fn snapshot_header(snapshot: &WorldSnapshot) -> usize {
    WorldSnapshot {
        tick: snapshot.tick,
        players: Vec::new(),
    }
    .encode_to_vec()
    .len()
}
