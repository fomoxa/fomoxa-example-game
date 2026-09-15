use std::collections::BTreeMap;
use std::f32::consts::{PI, TAU};

use fomoxa_net::PeerId;

use crate::models::protocol::{PlayerInput, PlayerState, WorldSnapshot};

pub const PLANE_HALF_SIZE: f32 = 10.0;
pub const TICK_RATE: u16 = 30;
pub const MOVE_SPEED: f32 = 5.0;
pub const GRAVITY: f32 = 20.0;
pub const JUMP_SPEED: f32 = 7.0;
pub const MAX_LOOK_PITCH: f32 = 1.5;

const SPAWN_RADIUS: f32 = 3.0;
const SPAWN_ANGLE_STEP: f32 = 2.399_963;
const PALETTE: [u32; 8] = [
    0xE6194BFF, 0x3CB44BFF, 0xFFE119FF, 0x4363D8FF, 0xF58231FF, 0x911EB4FF, 0x46F0F0FF,
    0xF032E6FF,
];

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub id: u32,
    pub client_kind: u8,
    pub display_name: String,
    pub color: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub look_yaw: f32,
    pub look_pitch: f32,
    move_x: f32,
    move_z: f32,
    vertical_velocity: f32,
    jump_held: bool,
    last_sequence: u32,
}

impl Player {
    pub fn is_grounded(&self) -> bool {
        self.position_y <= 0.0
    }
}

#[derive(Debug, Default)]
pub struct World {
    players: BTreeMap<PeerId, Player>,
    next_player_id: u32,
    tick: u32,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn player(&self, peer: PeerId) -> Option<&Player> {
        self.players.get(&peer)
    }

    pub fn peers(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.players.keys().copied()
    }

    pub fn join(&mut self, peer: PeerId, client_kind: u8, display_name: String) -> u32 {
        if let Some(existing) = self.players.get(&peer) {
            return existing.id;
        }

        self.next_player_id += 1;
        let id = self.next_player_id;
        let (position_x, position_z) = spawn_position(id);
        self.players.insert(
            peer,
            Player {
                id,
                client_kind,
                display_name,
                color: PALETTE[(id as usize - 1) % PALETTE.len()],
                position_x,
                position_y: 0.0,
                position_z,
                look_yaw: 0.0,
                look_pitch: 0.0,
                move_x: 0.0,
                move_z: 0.0,
                vertical_velocity: 0.0,
                jump_held: false,
                last_sequence: 0,
            },
        );
        id
    }

    pub fn leave(&mut self, peer: PeerId) -> Option<Player> {
        self.players.remove(&peer)
    }

    pub fn apply_input(&mut self, peer: PeerId, input: &PlayerInput) -> bool {
        let Some(player) = self.players.get_mut(&peer) else {
            return false;
        };
        if input.sequence < player.last_sequence {
            return false;
        }

        let (move_x, move_z) = bounded_direction(input.move_x, input.move_z);
        player.last_sequence = input.sequence;
        player.move_x = move_x;
        player.move_z = move_z;
        player.jump_held = input.jump;
        if input.look_yaw.is_finite() {
            player.look_yaw = wrap_angle(input.look_yaw);
        }
        if input.look_pitch.is_finite() {
            player.look_pitch = input.look_pitch.clamp(-MAX_LOOK_PITCH, MAX_LOOK_PITCH);
        }
        true
    }

    pub fn step(&mut self, dt: f32) {
        self.tick = self.tick.wrapping_add(1);
        for player in self.players.values_mut() {
            player.position_x = (player.position_x + player.move_x * MOVE_SPEED * dt)
                .clamp(-PLANE_HALF_SIZE, PLANE_HALF_SIZE);
            player.position_z = (player.position_z + player.move_z * MOVE_SPEED * dt)
                .clamp(-PLANE_HALF_SIZE, PLANE_HALF_SIZE);

            if player.is_grounded() && player.jump_held {
                player.vertical_velocity = JUMP_SPEED;
            }
            player.vertical_velocity -= GRAVITY * dt;
            player.position_y += player.vertical_velocity * dt;
            if player.position_y <= 0.0 {
                player.position_y = 0.0;
                player.vertical_velocity = 0.0;
            }
        }
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            tick: self.tick,
            players: self
                .players
                .values()
                .map(|player| PlayerState {
                    player_id: player.id,
                    client_kind: player.client_kind,
                    color: player.color,
                    position_x: player.position_x,
                    position_z: player.position_z,
                    position_y: player.position_y,
                    look_yaw: player.look_yaw,
                    look_pitch: player.look_pitch,
                })
                .collect(),
        }
    }
}

fn spawn_position(player_id: u32) -> (f32, f32) {
    let angle = player_id as f32 * SPAWN_ANGLE_STEP;
    (SPAWN_RADIUS * angle.cos(), SPAWN_RADIUS * angle.sin())
}

fn bounded_direction(move_x: f32, move_z: f32) -> (f32, f32) {
    let axis = |value: f32| if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 };
    let (x, z) = (axis(move_x), axis(move_z));
    let length = (x * x + z * z).sqrt();
    if length > 1.0 {
        (x / length, z / length)
    } else {
        (x, z)
    }
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEER_A: PeerId = PeerId(1);
    const PEER_B: PeerId = PeerId(2);

    fn input(sequence: u32, move_x: f32, move_z: f32) -> PlayerInput {
        PlayerInput {
            sequence,
            move_x,
            move_z,
            ..PlayerInput::default()
        }
    }

    fn jumping(sequence: u32) -> PlayerInput {
        PlayerInput {
            jump: true,
            ..input(sequence, 0.0, 0.0)
        }
    }

    fn looking(sequence: u32, look_yaw: f32, look_pitch: f32) -> PlayerInput {
        PlayerInput {
            look_yaw,
            look_pitch,
            ..input(sequence, 0.0, 0.0)
        }
    }

    fn joined_world() -> World {
        let mut world = World::new();
        world.join(PEER_A, 2, "a".to_owned());
        world
    }

    fn player_a(world: &World) -> &Player {
        world.player(PEER_A).unwrap()
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-3, "{actual} != {expected}");
    }

    #[test]
    fn joining_assigns_distinct_ids_and_colors() {
        let mut world = World::new();
        let first = world.join(PEER_A, 2, "a".to_owned());
        let second = world.join(PEER_B, 0, "b".to_owned());

        assert_ne!(first, second);
        assert_ne!(world.player(PEER_A).unwrap().color, world.player(PEER_B).unwrap().color);
        assert_eq!(world.player_count(), 2);
    }

    #[test]
    fn joining_twice_from_one_peer_keeps_the_same_player() {
        let mut world = joined_world();
        let first = player_a(&world).id;

        assert_eq!(world.join(PEER_A, 2, "again".to_owned()), first);
        assert_eq!(world.player_count(), 1);
    }

    #[test]
    fn input_moves_the_player_at_move_speed() {
        let mut world = joined_world();
        let start_x = player_a(&world).position_x;

        world.apply_input(PEER_A, &input(1, 1.0, 0.0));
        world.step(0.5);

        assert_close(player_a(&world).position_x, start_x + MOVE_SPEED * 0.5);
    }

    #[test]
    fn diagonal_input_is_normalized() {
        let mut world = joined_world();
        let before = player_a(&world).clone();

        world.apply_input(PEER_A, &input(1, 1.0, 1.0));
        world.step(0.1);

        let after = player_a(&world);
        let travelled = ((after.position_x - before.position_x).powi(2)
            + (after.position_z - before.position_z).powi(2))
        .sqrt();
        assert_close(travelled, MOVE_SPEED * 0.1);
    }

    #[test]
    fn position_is_clamped_to_the_plane() {
        let mut world = joined_world();

        world.apply_input(PEER_A, &input(1, 1.0, -1.0));
        world.step(100.0);

        assert_close(player_a(&world).position_x, PLANE_HALF_SIZE);
        assert_close(player_a(&world).position_z, -PLANE_HALF_SIZE);
    }

    #[test]
    fn non_finite_input_does_not_move_the_player() {
        let mut world = joined_world();
        let before = player_a(&world).clone();

        world.apply_input(PEER_A, &input(1, f32::NAN, f32::INFINITY));
        world.step(1.0);

        assert_close(player_a(&world).position_x, before.position_x);
        assert_close(player_a(&world).position_z, before.position_z);
    }

    #[test]
    fn stale_input_is_ignored() {
        let mut world = joined_world();

        assert!(world.apply_input(PEER_A, &input(5, 1.0, 0.0)));
        assert!(!world.apply_input(PEER_A, &input(4, -1.0, 0.0)));

        let start_x = player_a(&world).position_x;
        world.step(0.1);
        assert!(player_a(&world).position_x > start_x);
    }

    #[test]
    fn input_from_a_peer_that_never_joined_is_rejected() {
        let mut world = joined_world();

        assert!(!world.apply_input(PEER_B, &input(1, 1.0, 0.0)));
    }

    #[test]
    fn leaving_removes_the_player_from_the_snapshot() {
        let mut world = joined_world();
        world.join(PEER_B, 0, "b".to_owned());

        let left = world.leave(PEER_A).unwrap();

        let snapshot = world.snapshot();
        assert_eq!(snapshot.players.len(), 1);
        assert!(snapshot.players.iter().all(|state| state.player_id != left.id));
    }

    #[test]
    fn snapshot_reports_the_current_tick() {
        let mut world = joined_world();
        world.step(0.1);
        world.step(0.1);

        assert_eq!(world.snapshot().tick, 2);
        assert_eq!(world.tick(), 2);
    }

    #[test]
    fn a_player_stands_on_the_plane_without_jumping() {
        let mut world = joined_world();

        world.step(1.0);

        assert_eq!(player_a(&world).position_y, 0.0);
        assert!(player_a(&world).is_grounded());
    }

    #[test]
    fn holding_jump_on_the_ground_launches_the_player() {
        let mut world = joined_world();

        world.apply_input(PEER_A, &jumping(1));
        world.step(0.05);

        assert!(player_a(&world).position_y > 0.0);
    }

    #[test]
    fn a_jump_peaks_near_the_ballistic_height() {
        let mut world = joined_world();
        world.apply_input(PEER_A, &jumping(1));
        world.step(0.001);
        world.apply_input(PEER_A, &input(2, 0.0, 0.0));

        let mut highest = 0.0f32;
        for _ in 0..1000 {
            world.step(0.001);
            highest = highest.max(player_a(&world).position_y);
        }

        let expected = JUMP_SPEED * JUMP_SPEED / (2.0 * GRAVITY);
        assert!((highest - expected).abs() < 0.02, "peak {highest}, expected {expected}");
    }

    #[test]
    fn a_released_jump_lands_and_stays_on_the_plane() {
        let mut world = joined_world();
        world.apply_input(PEER_A, &jumping(1));
        world.step(0.01);
        world.apply_input(PEER_A, &input(2, 0.0, 0.0));

        for _ in 0..200 {
            world.step(0.01);
        }

        assert_eq!(player_a(&world).position_y, 0.0);
    }

    #[test]
    fn holding_jump_keeps_bouncing() {
        let mut world = joined_world();
        world.apply_input(PEER_A, &jumping(1));

        let mut takeoffs = 0;
        let mut was_grounded = true;
        for _ in 0..200 {
            world.step(0.01);
            let grounded = player_a(&world).is_grounded();
            if was_grounded && !grounded {
                takeoffs += 1;
            }
            was_grounded = grounded;
        }

        assert!(takeoffs >= 2, "took off {takeoffs} times in 2 seconds");
    }

    #[test]
    fn holding_jump_in_the_air_does_not_add_speed() {
        let mut world = joined_world();
        world.apply_input(PEER_A, &jumping(1));
        world.step(0.1);
        let rising_speed = player_a(&world).vertical_velocity;

        world.step(0.1);

        assert_close(player_a(&world).vertical_velocity, rising_speed - GRAVITY * 0.1);
    }

    #[test]
    fn look_yaw_is_wrapped_and_pitch_is_clamped() {
        let mut world = joined_world();

        world.apply_input(PEER_A, &looking(1, TAU + 0.5, 3.0));

        assert_close(player_a(&world).look_yaw, 0.5);
        assert_close(player_a(&world).look_pitch, MAX_LOOK_PITCH);
    }

    #[test]
    fn non_finite_look_keeps_the_previous_direction() {
        let mut world = joined_world();
        world.apply_input(PEER_A, &looking(1, 0.5, -0.25));

        world.apply_input(PEER_A, &looking(2, f32::NAN, f32::NEG_INFINITY));

        assert_close(player_a(&world).look_yaw, 0.5);
        assert_close(player_a(&world).look_pitch, -0.25);
    }

    #[test]
    fn snapshot_carries_height_and_look_direction() {
        let mut world = joined_world();
        world.apply_input(
            PEER_A,
            &PlayerInput {
                jump: true,
                ..looking(1, -1.0, 0.75)
            },
        );
        world.step(0.05);

        let state = &world.snapshot().players[0];
        assert!(state.position_y > 0.0);
        assert_close(state.look_yaw, -1.0);
        assert_close(state.look_pitch, 0.75);
    }
}
