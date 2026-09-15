# Fomoxa Example Protocol

The source of truth for all model declarations (Rust, GDScript, and subsequent clients). Each language re-declares the model according to the table below, without copying files from other languages, and then runs `fomoxac generate` individually.

- The only codec: `game`.
- Transport: TCP, default port `9321`.
- Current schema fingerprint: `0xC179C08CBF309660` - all clients must produce exactly this value (`tools/check-fingerprints.sh`).
- History: `0x23F616B3DDE5D0E8` is the version without jumping and look direction. The current version only appends fields to the end of `PlayerInput` and `PlayerState`, so `fomoxac` evaluates it as COMPATIBLE: older clients can still handshake, and missing fields are read as 0/false.

## Coordinate conventions

Every engine must convert to this system before sending and after receiving (Unity is a left-handed system, requires flipping the Z axis).

- Right-handed system, Y points up, the plane lies at `y = 0`.
- `look_yaw` (radians) rotates around +Y: `0` means looking towards −Z, positive is counter-clockwise when viewed from above. Horizontal view vector: `(-sin(yaw), 0, -cos(yaw))`.
- `look_pitch` (radians) rotates around the character's horizontal axis: positive means looking up, bounded by `[-1.5, 1.5]`.
- `move_x`, `move_z` are directions along the world axes, not the look direction: first-person clients rotate WASD according to `look_yaw` before sending.
- `position_y` is the height of the character's feet; the Godot client's eyes are placed at `position_y + 1.5`.
- Unreal (Z up, left-handed, cm): `X = -z·100`, `Y = x·100`, `Z = y·100`; Unreal yaw (degrees) is `-look_yaw`, pitch retains its sign.
- Kaiju (right-handed, Y up, meters, forward is −Z) matches this system: no axis swap, no angle sign change.

## Rules for fingerprint matching

1. The model name and codec name must match exactly character by character (the message id is hashed from `<Model>.game`).
2. The field order and wire types must perfectly match the table.
3. The field names are folded (lowercase, stripping `_`, `-`, space): `player_id`, `PlayerId`, `PlayerID`, `playerId` are all valid.
4. Do not use extern types (for instance, `Vector3` of `unity-codecs`); only use primitives and models in the table.

## Models

### `ClientHello` - client → server, sent once after the `Ready` handshake

| # | Field | Wire type | Meaning |
|---|---|---|---|
| 0 | `client_kind` | `u8` | client kind, see table below |
| 1 | `display_name` | `string` | display name |

### `Welcome` - server → client, responds to `ClientHello`

| # | Field | Wire type | Meaning |
|---|---|---|---|
| 0 | `player_id` | `u32` | this client's own player id |
| 1 | `plane_half_size` | `f32` | the plane is `[-h, h]` on the X and Z axes |
| 2 | `tick_rate` | `u16` | number of simulation ticks per second |

### `PlayerInput` - client → server, approximately every tick

| # | Field | Wire type | Meaning |
|---|---|---|---|
| 0 | `sequence` | `u32` | monotonically increasing; input with a lower sequence than the previous is dropped |
| 1 | `move_x` | `f32` | `[-1, 1]`, positive is +X |
| 2 | `move_z` | `f32` | `[-1, 1]`, positive is +Z |
| 3 | `jump` | `bool` | jump key is being held |
| 4 | `look_yaw` | `f32` | desired look direction, see coordinate conventions |
| 5 | `look_pitch` | `f32` | desired look direction, see coordinate conventions |

### `PlayerState` - only resides within `WorldSnapshot`

| # | Field | Wire type | Meaning |
|---|---|---|---|
| 0 | `player_id` | `u32` | |
| 1 | `client_kind` | `u8` | |
| 2 | `color` | `u32` | RGBA8888, assigned by server |
| 3 | `position_x` | `f32` | |
| 4 | `position_z` | `f32` | |
| 5 | `position_y` | `f32` | height of the feet, `0` is standing on the plane |
| 6 | `look_yaw` | `f32` | wrapped by server to `[-π, π)` |
| 7 | `look_pitch` | `f32` | clamped by server to `[-1.5, 1.5]` |

### `WorldSnapshot` - server → all joined clients, every tick

| # | Field | Wire type | Meaning |
|---|---|---|---|
| 0 | `tick` | `u32` | |
| 1 | `players` | `Array<PlayerState>` | all online players |

## `client_kind`

| Value | Client |
|---|---|
| 0 | headless bot |
| 1 | Unity |
| 2 | Godot |
| 3 | Unreal |
| 4 | Kaiju |

## Server behavior

- Simulation at 30 Hz, velocity of 5 units/second, `plane_half_size = 10`.
- The movement direction is clamped to `[-1, 1]` per axis, normalized if length > 1; non-finite values are treated as 0.
- Jumping: gravity `20`, jump velocity `7` (peak is around `1.22`). Only takes off when standing on the plane; if `jump` is still being held upon touching the ground, jumps again immediately. Holding `jump` in mid-air does not add force.
- Look direction: server receives `look_yaw`/`look_pitch` from the latest input, wraps/clamps them and broadcasts them in `PlayerState`; non-finite values are ignored, retaining the previous direction.
- A player is created upon receiving `ClientHello`, and destroyed when the peer disconnects.
- The client only renders based on the latest `WorldSnapshot`, with no client-side simulation.
