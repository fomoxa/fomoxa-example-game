# Fomoxa Example Game

Fomoxa cross-language demo: authoritative Rust server, clients in multiple engines joining and seeing each other simultaneously. Full plan in [PLAN.md](PLAN.md), protocol in [protocol/PROTOCOL.md](protocol/PROTOCOL.md).

Currently features: Rust server, headless Rust bot, Godot client (GDScript), Unity client (C#), Unreal client (C++), Kaiju client (Go).

## Benchmark

This benchmark aims to answer a single question: Is Fomoxa the bottleneck?

No. Encoding snapshots consumes under 0.1% of the tick budget: 0.02 ms for 1000 players and 0.04 ms for 2000 players, out of a 33.3 ms budget for a 30 Hz tick. The ceiling is hit by socket write syscalls - an OS limit and the cost of broadcasting full state to everyone, not a protocol limit: at 2000 players, 48.15 out of 53.5 ms per tick (90%) is spent in the `send` loop, while the codec remains virtually free.

A single-threaded Rust server maintains exactly 30 Hz for 1000 players with 0.57 vCPU and 85 MiB of RAM, while each player receives the full state of 999 others, 30 times per second.

Headless bots join the same server, each sending input at 30 Hz and receiving `WorldSnapshot` at 30 Hz. All figures below are from a single run of `tools/bench.sh --bots="100 500 1000 2000" --seconds=20 --warmup=5`.

### Stable 30 Hz Region

| Bots | p50 Latency | p95 | p99 | Snapshots/s | Received | Outbound Payload | Snapshot Size | Server CPU | Server RSS |
|---|---|---|---|---|---|---|---|---|---|
| 100 | 16.8 ms | 32.0 ms | 33.6 ms | 3,000 | 100% | 8 MiB/s | 2.9 KB | 0.05 vCPU | 8 MiB |
| 500 | 18.6 ms | 33.8 ms | 36.0 ms | 15,001 | 100% | 208 MiB/s | 14.5 KB | 0.26 vCPU | 36 MiB |
| 1000 | 22.6 ms | 39.7 ms | 43.6 ms | 30,001 | 100% | 830 MiB/s | 29.0 KB | 0.57 vCPU | 85 MiB |

Up to 1000 bots: the tick rate holds steady at 30.0 Hz, zero snapshots are dropped (`dropped 0`), and every bot receives 30.0/s.

### Breaking Point

| Bots | p50 Latency | p95 | p99 | Server Tick | Snapshots/s | Received | Outbound Payload | Server CPU | Server RSS |
|---|---|---|---|---|---|---|---|---|---|
| 2000 | 44.0 ms | 71.1 ms | 78.2 ms | 18.0 Hz | 35,810 | 60% | 1,981 MiB/s | 0.96 vCPU | 237 MiB |

2000 bots successfully join and none are disconnected, but the server can no longer maintain 30 Hz. The deficit is not due to dropped packets but slower ticks: each bot receives 17.9/s, exactly matching the actual tick rate (18.0 Hz), so `60% of 30 Hz` means "the server only produces 18 snapshots/second for each peer", not "37% of snapshots are dropped".

### Where the Tick Budget Goes

`fomoxa-example-server --profile` measures each phase in the tick loop (tick budget at 30 Hz is 33.3 ms):

| Bots | Tick | Read + Decode Input | Simulation | Encode Snapshot | Send Snapshot | Total |
|---|---|---|---|---|---|---|
| 100 | 30.0 Hz | 1.13 ms | 0.00 ms | 0.01 ms | 0.46 ms | 1.6 ms |
| 500 | 30.0 Hz | 4.22 ms | 0.01 ms | 0.01 ms | 4.12 ms | 8.4 ms |
| 1000 | 30.0 Hz | 5.52 ms | 0.02 ms | 0.02 ms | 13.23 ms | 18.8 ms |
| 2000 | 18.0 Hz | 5.26 ms | 0.02 ms | 0.04 ms | 48.15 ms | 53.5 ms |

Direct answer to "does it break at encoding or sending": at sending. At 2000 bots, 48.15 / 53.5 ms of a tick is spent in the `send` loop for 2000 peers; encoding takes only 0.04 ms, or 0.07% of the tick. 53.5 ms per tick equates to 18.7 Hz, which matches the measured 18.0 Hz - the server fails to broadcast snapshots within 33.3 ms, causing tick slippage, rather than failing to process inputs in time (input processing only takes 5.26 ms and bots still send a full 59,267 inputs/s).

In terms of bytes, the single-thread throughput ceiling is quite stable: 1000 peers × 29 KB in 13.23 ms and 2000 peers × 58 KB in 48.15 ms both equate to approximately 2.2 GB/s.

This 2.2 GB/s is the sender-side cost of `write()` - the syscall plus the copy into the kernel socket buffer - not the loopback bandwidth: at steady state, no snapshots are dropped due to backpressure, meaning `write()` never blocks waiting for the peer to read, it only pays the copy cost. Extrapolating from the 1000 and 2000 peer levels (13.23 µs for 29 KB, 24.08 µs for 58 KB per call), each `write()` costs roughly a fixed 2.4 µs + 0.37 ns per byte (≈ 2.7 GB/s during the copy): for small snapshots, the syscall dominates; for large snapshots, the copy dominates.

With full snapshots (29 bytes/player), 30 Hz, and a single thread, the theoretical ceiling is `30 × 29 × n² = 2.2 GB/s` → roughly 1,600 players; empirical measurements show 1000 holding steady and 2000 dropping to 18 Hz, exactly as predicted.

The `dropped` count in the log only increases while the 2000 bots are actively connecting (22,496 times), then stabilizes - no snapshots are dropped during steady state.

### What "0.96 vCPU" Means

CPU is measured as `utime + stime` from `/proc/<pid>/stat`, divided by real time: the unit is one logical CPU (one hyperthread) as seen by the guest, not a physical core.

- Test machine: 13th Gen Intel Core i5-13500HX, physical chip has 14 physical cores (6 P-cores + 8 E-cores) and 20 logical CPUs.
- Inside WSL2, the guest reports a synthetic topology: 20 logical CPUs arranged as 10 cores × 2 threads. Mapping to P-cores or E-cores is handled by Hyper-V and Windows; it is invisible to the guest and cannot be pinned, so it is impossible to state whether that vCPU is a P-core or E-core. This is a limitation of the measurement that must be noted when comparing with bare metal metrics.
- The server runs on a single thread (`Threads: 1` in `/proc/<pid>/status`), so 0.96–1.00 vCPU at 2000 bots means that specific thread is saturated. The ceiling here is a software limit (single thread), not a hardware limit: the remaining 19 vCPUs are idle.

### Isolated Codec Cost

```sh
server/target/release/fomoxa-example-bench --codec 1000
```

| Snapshot | Encode | Decode |
|---|---|---|
| 100 players (2,908 B) | 1.0 µs · 9.7 ns/player · 2.80 GiB/s | 2.7 µs · 26.8 ns/player · 1.01 GiB/s |
| 1000 players (29,008 B) | 10.3 µs · 10.3 ns/player · 2.61 GiB/s | 25.6 µs · 25.6 ns/player · 1.05 GiB/s |

This measures the pure loop without sockets, including `Vec` allocation per pass. This explains why the encode column in the tick breakdown is nearly zero: encoding a 1000-player snapshot takes 10 µs, whereas sending it to 1000 peers takes 13,000 µs.

### Baseline: Protobuf Comparison

Three baselines in [baseline/](baseline/), using `prost` (derive, no `protoc` required), sharing the same simulation logic (the demo's `World`) and same measurement code (the `measure` module shared by all load generators):

| Baseline | Replaces What | Answers What Question |
|---|---|---|
| 1. codec-only | only compares `fomoxac` vs `prost`, no sockets | how much faster/slower is the Fomoxa codec compared to Protobuf? |
| 2. `protobuf-net` | keeps fomoxa-net intact, only swaps the payload to protobuf | how much does the system change when swapping the codec? |
| 3. `protobuf-tcp` | raw TCP + custom framing (5-byte header) + protobuf | what is the overhead of the fomoxa-net session layer? |

```sh
tools/baseline.sh
tools/baseline.sh --bots=2000 --seconds=15 --repeats=1
```

Baseline 1 - codec (1000 players, 20,000 iterations, median of 3 runs):

| Codec | Encode | Decode | Bytes per player |
|---|---|---|---|
| `fomoxac` | 10.2 µs · 10.2 ns/player · 2.66 GiB/s | 19.1 µs · 19.1 ns/player · 1.42 GiB/s | 29.0 |
| `prost` (protobuf) | 21.9 µs · 21.9 ns/player · 1.48 GiB/s | 47.6 µs · 47.6 ns/player · 0.68 GiB/s | 34.9 |

The Fomoxa codec is ~2.1x faster at encoding, ~2.5x faster at decoding, and 17% smaller. Reason for the smaller size: Fomoxa's wire format omits field tags, whereas protobuf incurs 1 byte of tag overhead for each field per player.

Baselines 2 and 3 - 1000 bots, 20 seconds:

| Stack | p50 | p95 | p99 | Received | Snapshot Size | Encode/tick | Send/tick | Server CPU | Server RSS |
|---|---|---|---|---|---|---|---|---|---|
| fomoxa | 22.7 ms | 39.4 ms | 43.2 ms | 100% | 29,008 B | 0.03 ms | 12.91 ms | 0.56 vCPU | 84 MiB |
| protobuf-net | 23.0 ms | 40.1 ms | 43.8 ms | 100% | 30,137 B | 0.09 ms | 13.34 ms | 0.57 vCPU | 86 MiB |
| protobuf-tcp | 22.9 ms | 39.8 ms | 43.8 ms | 100% | 30,135 B | 0.09 ms | 13.00 ms | 0.54 vCPU | 40 MiB |

At 1000 players, the three stacks are indistinguishable: a 0.3 ms delta in p50, 0.6 ms delta in p99, and a 0.03 vCPU difference - exactly as predicted by the tick breakdown, since a codec that is twice as fast only saves 0.06 ms in a 33.3 ms tick. This is bilateral evidence: the Fomoxa codec is faster than Protobuf, and in this workload, that fact does not change anything.

Two notable details from the table:

- The on-the-wire protobuf snapshot is only 30,137 B, not 34,862 B like the micro-benchmark, because protobuf omits zero-value fields, and in the swarm, 900 bots send `look_pitch = 0`, with the majority standing on the floor (`position_y = 0`). This advantage is still insufficient to beat Fomoxa's 29,008 B.
- `protobuf-tcp` consumes only 40 MiB RSS because the custom `Link` allocates a single 64 KiB read buffer shared across all peers, whereas fomoxa-net allocates a 64 KiB `recv_buf` per connection (`vec![0; config.recv_buffer_size]`). The 44 MiB difference aligns exactly with 1000 × 64 KiB minus untouched pages.

At 2000 players (saturated, each stack run 2 times):

| Stack | Tick Achieved | p99 |
|---|---|---|
| fomoxa | 18.3 and 17.3 Hz | 82.4 and 81.5 ms |
| protobuf-net | 17.4 and 18.8 Hz | 81.3 and 76.1 ms |
| protobuf-tcp | 20.1 and 20.9 Hz | 74.7 and 71.8 ms |

When hitting the ceiling, differences emerge, and they are unfavorable to fomoxa-net: the raw TCP stack maintains roughly 10–15% higher tick rate, despite its protobuf payload being 3% larger. The cause is evident from the phase table: both phases are cheaper - `events` 3.8–4.0 ms compared to 5.0–5.5 ms, `send` 41.8–43.4 ms compared to 47.5–50.3 ms. The fomoxa-net session layer (11-byte framing, probes/heartbeats, outbox discipline, per-peer bookkeeping) incurs this exact overhead. Between `fomoxa` and `protobuf-net`, the difference at the breaking point falls within run-to-run variance, meaning swapping the codec does not shift the bottleneck.

What this baseline is not fair about or does not answer:

- `protobuf-tcp` lacks handshake, schema negotiation, probe/heartbeat, and message id - this is intentional: it serves as a floor, not a functionally equivalent competitor.
- The baseline decodes protobuf into `prost` structs, then copies 6 fields into the demo's model to call `apply_input`; for the snapshot direction, it builds the protobuf struct directly from the `World`, avoiding additional allocations.
- This is Protobuf in Rust via prost. It implies nothing about protobuf in C++/Go/C#, nor does it provide comparisons against FlatBuffers, Cap'n Proto, or other engine protocols.
- This remains a single-machine loopback test. Over a real network, the 3% byte overhead of protobuf becomes meaningful.

### What These Numbers Mean to You

If you are building a game and selecting a netcode stack. Single-threaded, 1000 concurrent players, each receiving the full state of 999 others 30 times per second, p99 43.6 ms. That p99 figure primarily represents tick wait time (one tick is 33 ms): to reduce latency, you must increase the tick rate or add client-side prediction; swapping the protocol will yield no benefit. If your game does not broadcast full state to everyone - i.e., nearly all real games - then 1000 is not your ceiling.

If you are evaluating Fomoxa against another stack. The wire format costs 29 bytes per player (8 fields, no field tags on the wire), encodes at 10 ns/player, decodes at 19 ns/player - 2.1x faster than `prost` for encoding, 2.5x faster for decoding, and 17% smaller (see baseline table above). However, that same table shows that at 1000 players, systems using Fomoxa and Protobuf differ by 0.3 ms p50 and 0.03 vCPU, rendering them indistinguishable. The critical comparison lies in the I/O model and sending strategy, not serialization speed - and there, a raw TCP stack outperforms fomoxa-net by 10–15% under saturation.

If you are operating it. 1000 connections consume 85 MiB RSS, roughly 85 KB per connection, largely dictated by the default 64 KiB `recv_buffer_size` in `Config` - each `Connection` allocates its own buffer. Testing with `recv_buffer_size = 8 KiB`: RSS drops to 49 MiB while maintaining a 30 Hz tick, 43.3 ms p99, and 0.56 vCPU, sacrificing nothing. This is the fastest way to reduce RAM footprint when the server only processes small messages like `PlayerInput`. CPU usage is merely 0.57 vCPU at 1000 players, so your primary concern is bandwidth (830 MiB/s at 1000 players, scaling quadratically), not CPU. Because the server is single-threaded, adding cores natively will not help: you must shard players across multiple processes or partition the `send` loop.

If you are contributing to the Fomoxa runtime. The tick breakdown points to a single optimization target: the `send` loop (48 ms out of 53 ms per tick at 2000 peers), specifically the number of `write()` calls and bytes copied - frame batching, `writev`, or multi-threaded dispatch. Encoding and decoding require no further optimization until this is resolved.

If you need numbers for a production environment. This is a single-machine loopback test with a single server process, worst-case fan-out, no NIC, no TLS, and no WAN latency. Using these numbers to conclude that "the codec is not the bottleneck" is valid; using them to predict concurrent players on production infrastructure is not - you must re-measure with your network and dispatch pattern.

### Test Machine and Reproducibility

| | |
|---|---|
| CPU | 13th Gen Intel Core i5-13500HX - 14 physical cores (6 P + 8 E), 20 logical CPUs |
| RAM | 7 GiB allocated to WSL2 |
| OS | WSL2, kernel 6.6.87.2-microsoft-standard-WSL2, on Windows |
| Network | intra-machine loopback, bypassing the NIC - thus MiB/s figures are not bounded by 1/10 Gbps limits, but also do not reflect real network conditions |
| Build | rustc 1.98.0, `cargo build --release` |
| Window | 20 seconds after a 5-second warmup, 30 Hz tick, 30 Hz input |

```sh
tools/bench.sh
tools/bench.sh --bots="100 1000" --seconds=30 --warmup=10 --port=9321
tools/bench.sh --bots=1000 --input-hz=120
```

- `tools/bench.sh` auto-builds, executes a fresh `fomoxa-example-server --quiet --profile` for each tier (RSS measured from zero), and outputs a markdown table. JSON and logs for each run are saved to `${TMPDIR:-/tmp}/fomoxa-example-bench/`.
- A higher `--input-hz` relative to the tick rate ensures fresher inputs when the tick runs, minimizing tick wait time and highlighting pure processing overhead.
- Latency represents the round-trip of the input itself: from the moment the bot sends `PlayerInput` to when it receives a `WorldSnapshot` containing that exact input. The bot encodes a marker in `look_pitch` (512 distinct values spaced by 0.001, within a range the server does not clamp), and the server echoes it verbatim in `PlayerState`, allowing the bot to correlate the snapshot with the send time without adding fields to the protocol (the fingerprint remains unchanged). This measurement includes the wait time for the next tick: because the server processes only the most recent input before a tick, the theoretical floor is half a tick (16.7 ms) and the p99 cannot fall below one full tick (33 ms). Any latency beyond 33 ms is the true cost of encoding, TCP transmission, and decoding.
- By default, 100 evenly distributed bots decode snapshots and measure latency; the rest simply count bytes. The `per bot` row logs speeds for both groups separately to verify the load generator is not bottlenecked: at 2000 bots, both groups maintain 17.9/s, meaning decoding bots are no slower than counting bots.
- CPU/RSS is sampled from `/proc/<pid>/stat` and `/proc/<pid>/statm` every 200 ms, converted using `CLK_TCK = 100`. The load generator consumes 0.17 → 6.44 vCPU (3–7x more than the server as it drives 2000 sockets); beyond 2000 bots, it competes with the server for CPU time, introducing noise to the measurements.
- Payload is the byte count of the Fomoxa message, excluding the 11-byte header per frame.

### Context Required to Interpret These Numbers

- The demo broadcasts full snapshots to all players every tick, causing bandwidth to scale quadratically: 1000 players equates to 29 KB × 1000 peers × 30 Hz. This is a worst-case scenario, reflecting the demo's design rather than the runtime's limit. A production game would filter by visibility or send deltas, reducing bandwidth by orders of magnitude and supporting significantly higher concurrency.
- The server intentionally operates on a single thread for code readability. The `send` loop - the sole bottleneck - is entirely parallelizable; these figures do not represent architectural limits.
- Baselines exist, but only along one axis. Comparisons with Protobuf (`prost`) cover three tiers: pure codec, swapped codec in the same stack, and a raw TCP stack - see [Baseline: Protobuf Comparison](#baseline-protobuf-comparison). No comparisons have been made against other engine runtimes (Unity Netcode, Photon, ENet) or Fomoxa SDKs in other languages; doing so requires reimplementing this specific workload on those stacks and benchmarking on the same hardware.

## Requirements

| Tool | Notes |
|---|---|
| Rust (cargo) | builds the server and bot |
| `fomoxac` | `cargo install --git https://github.com/fomoxa/fomoxac` - required only when modifying models, see [Where to Get the Fomoxa Runtime](#where-to-get-the-fomoxa-runtime) |
| Godot 4.3 | this machine: `~/.local/opt/godot/Godot_v4.3-stable_linux.x86_64` |
| Unity 6000.5.7f1 | installed on Windows via Unity Hub; WSL requires `rsync` and `powershell.exe` |
| Unreal Engine 5.8 | installed on Windows; Visual Studio 2022 with the *Game development with C++* workload and .NET Framework 4.8 SDK component (missing this causes UBT to report *Could not find NetFxSDK install dir*) |
| Go 1.27+ | builds the Kaiju client and its smoke tests |
| Kaiju Engine | `tools/kaiju.sh setup` auto-clones; building requires cgo + Vulkan + X11 (see Kaiju client section) |

## Where to Get the Fomoxa Runtime

This repository is standalone: cloning it provides everything needed to build and run, without requiring an adjacent checkout of Fomoxa. Each runtime follows the distribution conventions of its target language ecosystem.

| Component | Source | In this repo |
|---|---|---|
| `fomoxa-net`, `fomoxa-attributes` - Rust server and bot | crates.io | declared in `server/Cargo.toml`, downloaded automatically via `cargo build` |
| `fomoxac` - generator | [fomoxa/fomoxac](https://github.com/fomoxa/fomoxac) | installed separately, not tracked in the repo |
| `github.com/fomoxa/go` - Kaiju | Go module proxy | declared in `clients/kaiju/go.mod`, downloaded automatically via `go build` - no vendor needed |
| `prost` - Protobuf baseline only | crates.io | declared in `baseline/Cargo.toml`; the main demo crate remains dependency-free outside Fomoxa |
| Godot addon | [fomoxa/godot](https://github.com/fomoxa/godot) | committed at `clients/godot/addons/fomoxa` |
| `Fomoxa.Net.dll` - Unity | [fomoxa/csharp](https://github.com/fomoxa/csharp) | committed at `clients/unity/Assets/Plugins` |
| C runtime + `net.hpp` - Unreal | [fomoxa/c](https://github.com/fomoxa/c) | committed at `clients/unreal/Source/FomoxaExample/ThirdParty/fomoxa` |

The three engine runtimes are committed directly to the repo because Godot, Unity, and Unreal lack native package managers to fetch them during the build process: Godot loads addons from `addons/`, Unity loads DLLs from `Assets/Plugins/`, and UBT compiles C source embedded in the module tree. Committing them also guarantees they match the version tested with this demo, insulating them from `main` drift in the upstream repositories.

`fomoxac` must be installed from git, not crates.io. Version `0.2.0` on crates.io generates C# code lacking an explicit cast (`reader.FieldAbsent() ? 0 : reader.ReadU8()`), resulting in an `int` ternary that fails to compile in Unity; commit `dbe12b5` on `main` fixes this by adding `(byte)0`, but a new release is not yet available. For the other three backends (Rust, GDScript, C++), the crates.io version generates code identical to what is committed here.

```sh
cargo install --git https://github.com/fomoxa/fomoxac
```

Each engine runtime has a dedicated vendor script: it performs a `git clone` from upstream, copies the necessary engine files, and logs the repository URL and commit hash to a `VERSION` file next to the runtime. Never manually copy from an adjacent Fomoxa checkout - doing so obscures the source commit.

```sh
tools/vendor-fomoxa-godot.sh
tools/vendor-fomoxa-csharp.sh
tools/vendor-fomoxa-c.sh

tools/vendor-fomoxa-c.sh 17cef16654bd90c92178b39f64f4157dba8962d5
```

Running without arguments defaults to `main`; supply a branch, tag, or commit hash to pin a specific version.

| Script | Writes to | Current Commit |
|---|---|---|
| `vendor-fomoxa-godot.sh` | `clients/godot/addons/fomoxa/` (+ `VERSION`, `LICENSE`) | `f97daf0` |
| `vendor-fomoxa-csharp.sh` | `clients/unity/Assets/Plugins/Fomoxa.Net.dll` (+ `Fomoxa.Net.VERSION.txt`) | `c984232` |
| `vendor-fomoxa-c.sh` | `clients/unreal/Source/FomoxaExample/ThirdParty/fomoxa/` (+ `VERSION`) and `Protocol/fomoxa.h` | `17cef16` |

- `vendor-fomoxa-csharp.sh` requires the .NET SDK because it builds `Fomoxa.Net` for `netstandard2.1` from source; on this WSL setup, `dotnet` lacks `libicu`, so the script automatically sets `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1`. A NuGet package for `Fomoxa.Net` 0.1.0 exists but is built from an older commit (`edc595e`), and since Unity cannot consume NuGet packages natively, the DLL must reside in `Assets/Plugins/`.
- Re-run the script and restart the engine to verify compilation remains successful: the Unity DLL and Unreal C runtime evolve alongside upstream.

## Running

```sh
cd server
cargo run --release --bin fomoxa-example-server
cargo run --release --bin fomoxa-example-server -- --addr 127.0.0.1:9321
```

By default, the server binds to `0.0.0.0:9321`. `--status-interval <seconds>` configures the status print interval (default 30, `0` disables). `--quiet` suppresses per-peer logging (connect / handshake / join / leave / roster) while retaining `status` output and warnings - recommended for tests with hundreds of clients, as `roster` dumps the full player list on every join. `--profile` adds a `profile` line adjacent to each `status` output, detailing the actual tick rate, total loop iterations per tick, and average duration of each phase (read input, simulate, encode, send).

Server logs (in UTC) indicate client entry/exit, source address, duration, and reason:

```
[09:40:04.450Z] INFO  connect     peer#1 from 127.0.0.1:38392 · 1 connections
[09:40:04.953Z] INFO  disconnect  peer#1 from 127.0.0.1:38392 after 0.5s during handshake: the peer closed the connection
[09:40:06.034Z] INFO  connect     peer#3 from 127.0.0.1:38414 · 2 connections
[09:40:06.036Z] INFO  handshake   peer#3 from 127.0.0.1:38414 accepted
[09:40:06.040Z] INFO  join        #2 godot "log-godot" from 127.0.0.1:38414 (peer#3)
[09:40:06.040Z] INFO  roster      online 2: #1 bot "log-bot" @127.0.0.1:38406 (peer#2), #2 godot "log-godot" @127.0.0.1:38414 (peer#3)
[09:40:09.040Z] INFO  leave       #2 godot "log-godot" from 127.0.0.1:38414 (peer#3) after 3.0s: the connection broke
[09:40:13.337Z] INFO  status      tick 900 · 0 connections · online 0 · snapshots 1782 sent, 0 dropped
```

| Label | When |
|---|---|
| `connect` | new TCP connection |
| `handshake` | schema negotiated; `WARN handshake ... refused` indicates fingerprint mismatch |
| `join` | client transmits `ClientHello`, including engine type and name |
| `leave` | active player disconnects, noting duration and reason |
| `disconnect` | connection drops prior to join (`during handshake` / `before sending ClientHello`) |
| `roster` | online list updated after every join/leave |
| `status` | periodic metrics; `dropped` indicates snapshots discarded due to slow peer consumption (previous frame still pending) |
| `profile` | `--profile` only: actual tick rate and per-phase timing |
| `WARN decode` / `WARN message` | unparseable client payload / unknown message id |

Godot Client: Open the editor or execute directly. The main scene `main.tscn` contains the environment, lighting, floor (`floor.gd` is `@tool` and visible in the editor), camera, and HUD; player blocks spawn dynamically based on snapshots.

```sh
GODOT=~/.local/opt/godot/Godot_v4.3-stable_linux.x86_64
$GODOT --path clients/godot
$GODOT --path clients/godot -- --host=127.0.0.1 --port=9321 --name=alice
```

Controls (first-person view):

| Key | Action |
|---|---|
| Left Click | lock mouse to window for view rotation |
| Mouse Move | rotate yaw / pitch |
| `WASD` / Arrows | move relative to viewing angle |
| Hold `Space` | jump continuously |
| `Esc` | release mouse lock |
| `R` | reconnect on disconnect |

Other players appear as headless blocks; the dark section indicates face direction, and the head tilts according to pitch.

Unity Client:

Unity cannot open projects stored on the WSL filesystem (reports *case sensitive file system*). `clients/` is bind-mounted from `D:\code\fomoxa-example-game\clients`, allowing Unity to open `D:\code\fomoxa-example-game\clients\unity` directly without syncing.

```sh
tools/unity.sh open
tools/unity.sh check
tools/unity.sh scene
tools/unity.sh smoke --seconds=4 --expect-players=2
FOMOXA_EXAMPLE_UNITY_PROJECT='E:\\code\\fomoxa-example-game\\clients\\unity' tools/unity.sh open
```

- Open `Assets/FomoxaExample/Scenes/Main.unity` and press Play. The scene contains a `Main Camera`, `Directional Light`, `Floor` (`FomoxaExampleFloor`, material in `Assets/FomoxaExample/Materials/`), and `Fomoxa Example` (`FomoxaExampleGame`: host/port/name/sensitivity exposed in Inspector). Player blocks spawn dynamically based on snapshots.
- `tools/unity.sh scene` (or menu Fomoxa Example → Rebuild Main Scene) reconstructs `Main.unity` and materials via `FomoxaExampleSceneBuilder`, overwriting manual edits. The Unity editor must be closed when running batchmode commands.
- Controls mirror the Godot client (click to lock, WASD, hold Space, Esc, R).
- When building a player, pass arguments like `--host=192.168.1.10 --port=9321 --name=bob`; `Main.unity` is pre-configured in Build Settings.
- `.meta` files must be committed: the scene references scripts and materials via GUIDs inside them.

Unreal Client (UE 5.8):

`clients/` is bind-mounted from `D:\code\fomoxa-example-game\clients`, meaning the Unreal project resides directly on the Windows drive, requiring no sync.

```sh
tools/unreal.sh build
tools/unreal.sh map
tools/unreal.sh smoke -host=127.0.0.1 -name=unreal-smoke
tools/unreal.sh open
tools/unreal.sh open -game -host=127.0.0.1 -port=9321 -name=carol
```

- `build` compiles `FomoxaExampleEditor Win64 Development` via `Build.bat`; the Unreal editor must be closed. Alternatively, open `clients/unreal/FomoxaExample.uproject` and select Yes when prompted to build the `FomoxaExample` module.
- The startup map is `/Game/Maps/FomoxaExample` (`Content/Maps/FomoxaExample.umap`), featuring an `Environment` (lighting, sky), `Floor` (grid floor, constructed via `OnConstruction` so visible in-editor), and a `View Camera`. Press Play to connect. Other maps function identically as the game mode is globally configured; if missing, the player controller auto-generates a floor or camera.
- `map` reconstructs the map via `Scripts/create_map.py` utilizing the `pythonscript` commandlet (requires `PythonScriptPlugin`, active only in editor builds), overwriting manual edits.
- `smoke` launches the game with `-nullrhi` until the first `WorldSnapshot` is received (requires active server), driven by `LogFomoxaExample` output. Logs output to `clients/unreal/Saved/Logs/`.
- Default host/port/name/sensitivity reside in `Config/DefaultGame.ini` under `[/Script/FomoxaExample.FomoxaExamplePlayerController]`; arguments `-host=`, `-port=`, and `-name=` override these.
- Controls mirror the Godot client. During PIE, `Esc` stops Play in the editor: use `Shift+F1` to release the mouse, or launch with `-game`/Standalone.
- Testing multiplayer in a single editor: Play → *Number of Players* = 2, *Net Mode* = *Play Standalone*; each instance runs as an independent client.
- The Fomoxa C runtime resides in `Source/FomoxaExample/ThirdParty/fomoxa`, fetched via `tools/vendor-fomoxa-c.sh` (see [Where to Get the Fomoxa Runtime](#where-to-get-the-fomoxa-runtime); the script also updates `Source/FomoxaExample/Protocol/fomoxa.h` for `fomoxac` annotation). It is compiled directly within the game module (PLAN §9 D3-A), linking against `ws2_32`. Do not edit these files manually. It compiles clean without warnings under MSVC 14.44.
- `Source/FomoxaExample/Network/` is pure C++17, devoid of Unreal dependencies; it has been compiled via g++ (`-Wall -Wextra -Wshadow -Werror`) and verified against the WSL server for join/move/jump/look operations.

Kaiju Client (Go):

Kaiju operates not as an imported library but as a framework: Go game code lives inside the engine's source tree (`src/`, under module `kaijuengine.com`), compiled via cgo. Therefore, `clients/kaiju` declares `module kaijuengine.com/fomoxa_example` - maintaining a stable import path whether inside this repo or copied into the engine - and `tools/kaiju.sh` orchestrates copying and building.

```sh
tools/kaiju.sh check
tools/kaiju.sh smoke -addr=127.0.0.1:9321 -name=kaiju-smoke -seconds=5 -expect-players=2
tools/kaiju.sh setup
tools/kaiju.sh build
tools/kaiju.sh run
```

- `check` runs `go vet` against engine-independent code and verifies fingerprints. `smoke` builds `cmd/smoke`, joins the real server, moves +X, jumps, and verifies the echoed look vector - without requiring the engine, similar to pre-UE validation for the Unreal client.
- `setup` clones [KaijuEngine/kaiju](https://github.com/KaijuEngine/kaiju) (including ~50 MB of prebuilt submodule binaries) into `~/.cache/kaiju-engine` and records the commit in `VERSION`; override with `KAIJU_ENGINE=<path>`.
- `build` copies `clients/kaiju/network` to `src/fomoxa_example/network`, copies `cmd/kaijugame/game.go` to `src/fomoxa_example_game.go`, deletes the engine's `src/main.test.go` (which also defines `getGame`), injects `github.com/fomoxa/go` into the engine's go.mod, and executes `go build -tags kaiju`.
- The Kaiju Engine shares the protocol's coordinate system (right-handed, Y-up, forward is −Z), meaning this client requires no axis conversion - unlike Unity and Unreal.
- Controls: hold Left Click and drag to rotate view, `WASD`/Arrows to move, `Space` to jump, `R` to reconnect. Host and name fallback to environment variables `FOMOXA_EXAMPLE_ADDR` and `FOMOXA_EXAMPLE_NAME`.
- Linux builds require engine C dependencies: `sudo apt install libx11-dev libxcursor-dev libxrandr-dev libasound2-dev libvulkan-dev`. `libvulkan-dev` is mandatory despite the presence of a loader: the engine strictly targets `dlopen("libvulkan.so")`, whereas WSL only provides `libvulkan.so.1`. Windows requires 64-bit Go + MinGW + Vulkan SDK per [Kaiju instructions](https://github.com/KaijuEngine/kaiju/blob/master/docs/engine/build_from_source.md).
- If root access is unavailable, extract these packages locally and set `KAIJU_SYSROOT=<path>` (pointing to `usr/include` and `usr/lib/x86_64-linux-gnu`); `build` and `run` inject appropriate cgo flags and `LD_LIBRARY_PATH`.
- Verified working on WSL2 + WSLg (Vulkan over Mesa): `tools/kaiju.sh build` then `tools/kaiju.sh run`, server logs `join #2 kaiju "kaiju-final"` and the bot observes 2 players. Inside WSL, the engine reports an ALSA error (`Cannot access file /usr/share/alsa/alsa.conf`) before continuing execution - audio simply fails.

Headless Bot (walks in circles, continuously jumps, nods):

```sh
cd server
cargo run --release --bin fomoxa-example-bot -- --name bot-1
```

## Testing

```sh
cd server && cargo test

tools/check-fingerprints.sh

GODOT=~/.local/opt/godot/Godot_v4.3-stable_linux.x86_64
$GODOT --headless --editor --quit --path clients/godot
cargo run --release --bin fomoxa-example-server &
cargo run --release --bin fomoxa-example-bot -- --seconds 6 --expect-players 2 &
$GODOT --headless --path clients/godot -s tests/smoke_test.gd -- --seconds=4 --expect-players=2
tools/unity.sh smoke --seconds=4 --expect-players=2
tools/unreal.sh smoke -host=127.0.0.1 -name=unreal-smoke
tools/kaiju.sh smoke -addr=127.0.0.1:9321 -name=kaiju-smoke -seconds=4 -expect-players=2

tools/bench.sh --bots=100 --seconds=6 --warmup=2
tools/baseline.sh --bots=100 --seconds=6 --warmup=2 --repeats=1 --rounds=2000
```

- The `--editor --quit` command executes once to register `class_name`s in Godot. Without a `.godot/` folder, `--import` in Godot 4.3 exits prematurely before scanning files.
- The Godot smoke test passes when: join succeeds, `WorldSnapshot` is received, expected player count matches, self-player translates +X, jump altitude exceeds `0.5`, and the server echoes the precise look vector.
- Executing the main scene in `--headless` outputs `mesh_get_surface_count ... Parameter "m" is null` due to a dummy renderer; this warning is benign.
- The Unity smoke test (`FomoxaExample.EditorTools.FomoxaExampleSmokeTest`) executes via editor batchmode rather than Play Mode: it verifies the protocol ↔ Unity coordinate transformation, instantiates a real `FomoxaExampleSession` to join, move, and jump, then validates the look vector identically to Godot. Output is logged under `FOMOXA-EXAMPLE-SMOKE:` inside `clients/unity/Logs/unity-smoke.log`.
- Initial `tools/unity.sh` runs incur a multi-minute penalty as Unity imports the full project.
- `tools/bench.sh` outputs `BENCH: PASS` if all bots connect, none drop, and latency samples are valid; `DEGRADED` indicates compromised metrics, referencing the `first loss` line for context. `tools/baseline.sh` performs identical workloads against three stacks and generates a comparison table.

## Modifying the Protocol

1. Modify [protocol/PROTOCOL.md](protocol/PROTOCOL.md).
2. Update models in `server/src/models/protocol.rs`, `clients/godot/network/models/`, `clients/unity/Assets/FomoxaExample/Networking/Models/`, and `clients/unreal/Source/FomoxaExample/Protocol/Models/`.
3. Execute `fomoxac generate` in `server/`, `clients/godot/`, `clients/unity/Assets/FomoxaExample/Networking/`, and `clients/unreal/Source/FomoxaExample/Protocol/`.
4. Run `tools/check-fingerprints.sh`, then update the fingerprint value in `PROTOCOL.md`.

## Structure

```
server/
  src/models/protocol.rs                      annotated model
  src/generated/                              fomoxac output
  src/world.rs                                pure simulation, includes unit tests
  src/peers.rs                                addresses, connection timing, roster for logs
  src/log.rs                                  log_info! / log_warn! with UTC timestamps
  src/main.rs                                 fomoxa-example-server
  src/bin/bot.rs                              fomoxa-example-bot
  src/bin/bench.rs                            fomoxa-example-bench: Fomoxa bot swarm + codec micro-benchmark
  src/measure.rs                              shared measurement code: windowing, latency, throughput, CPU/RSS, JSON
baseline/                                     Protobuf comparison (isolated crate, sole consumer of prost)
  src/protocol.rs                             protobuf model (prost derive, no protoc)
  src/framing.rs                              5-byte framing + non-blocking Link for raw TCP baseline
  src/bin/codec.rs                            baseline 1: pure prost encode/decode
  src/bin/net_server.rs, net_bench.rs         baseline 2: protobuf payload over fomoxa-net
  src/bin/tcp_server.rs, tcp_bench.rs         baseline 3: protobuf over raw TCP
clients/godot/
  addons/fomoxa/                              Fomoxa runtime for Godot, via tools/vendor-fomoxa-godot.sh (see VERSION)
  network/models/                             GDScript model
  network/generated/                          fomoxac output
  network/fomoxa_example_session.gd           connect, hello, input, snapshot
  main.tscn                                   main scene: environment, lights, floor, camera, HUD
  main.gd                                     connect, input, camera, player blocks
  floor.gd                                    grid floor, @tool enabled for editor preview
  tests/smoke_test.gd                         headless end-to-end test
clients/unity/
  Assets/Plugins/Fomoxa.Net.dll               Fomoxa runtime for .NET (netstandard2.1), built via tools/vendor-fomoxa-csharp.sh
  Assets/FomoxaExample/Networking/Models/     C# model
  Assets/FomoxaExample/Networking/Generated/  fomoxac output
  Assets/FomoxaExample/Networking/            FomoxaExampleSession, FomoxaExampleProtocol, FomoxaExampleMath (UnityEngine independent)
  Assets/FomoxaExample/Scenes/Main.unity      main scene, generated via tools/unity.sh scene
  Assets/FomoxaExample/Materials/             floor and grid materials
  Assets/FomoxaExample/Game/FomoxaExampleGame.cs  connect, input, camera, player blocks
  Assets/FomoxaExample/Game/FomoxaExampleFloor.cs grid floor driven by plane_half_size
  Assets/FomoxaExample/Game/FomoxaExampleAxes.cs  protocol (right-handed) ↔ Unity (left-handed) coordinate transform
  Assets/FomoxaExample/Editor/                batchmode smoke test, FomoxaExampleSceneBuilder
clients/kaiju/
  go.mod                                      module kaijuengine.com/fomoxa_example
  network/models/protocol.go                  Go model (annotated with //fomoxa:model + struct tags)
  network/generated/                          fomoxac output
  network/session.go                          connect, hello, input, snapshot
  network/math.go                             yaw-based forward vector, wrap/clamp look angles
  cmd/smoke/main.go                           headless smoke test, avoids engine dependency
  cmd/kaijugame/game.go                       Kaiju game: floor, avatar, camera, HUD (kaiju build tag)
clients/unreal/
  FomoxaExample.uproject
  Config/                                     global game mode, startup map, Enhanced Input, default host/port
  Content/Maps/FomoxaExample.umap             main map, generated via tools/unreal.sh map
  Scripts/create_map.py                       Python script mapping generation
  Source/FomoxaExample/Protocol/Models/       C++ model
  Source/FomoxaExample/Protocol/Generated/    fomoxac output
  Source/FomoxaExample/ThirdParty/fomoxa/     Fomoxa C runtime + net.hpp, via tools/vendor-fomoxa-c.sh (see VERSION)
  Source/FomoxaExample/Network/               FomoxaExampleSession, FomoxaExampleProtocol, FomoxaExampleMath (pure C++17)
  Source/FomoxaExample/Game/                  subsystem holding session, game mode, player controller, HUD, avatar, floor, environment
  Source/FomoxaExample/Game/FomoxaExampleAxes.h  protocol (Y-up, meters) ↔ Unreal (Z-up, cm) coordinate transform
tools/check-fingerprints.sh
tools/kaiju.sh                                check / smoke / setup / build / run Kaiju client
tools/bench.sh                                run server + 100/500/1000 bots, output benchmark table
tools/baseline.sh                             execute identical workload across 3 stacks (Fomoxa, protobuf+fomoxa-net, protobuf+TCP)
tools/unity.sh                                check / scene / smoke / open Windows-side Unity project
tools/unreal.sh                               build / map / smoke / open Windows-side Unreal project
tools/vendor-fomoxa-godot.sh                  git clone fomoxa/godot into clients/godot, log commit to VERSION
tools/vendor-fomoxa-csharp.sh                 git clone + build fomoxa/csharp into clients/unity, log commit to VERSION
tools/vendor-fomoxa-c.sh                      git clone fomoxa/c into clients/unreal, log commit to VERSION
```
<!--  
sudo mount --bind /mnt/d/code/fomoxa-example-game/clients /home/thanghd/code/fomoxa/fomoxa-example-game/clients
-->