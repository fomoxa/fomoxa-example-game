use fomoxa_example_server::models::protocol as fomoxa;
use fomoxa_example_server::world::{Player, World};
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct ClientHello {
    #[prost(uint32, tag = "1")]
    pub client_kind: u32,
    #[prost(string, tag = "2")]
    pub display_name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct Welcome {
    #[prost(uint32, tag = "1")]
    pub player_id: u32,
    #[prost(float, tag = "2")]
    pub plane_half_size: f32,
    #[prost(uint32, tag = "3")]
    pub tick_rate: u32,
}

#[derive(Clone, PartialEq, Message)]
pub struct PlayerInput {
    #[prost(uint32, tag = "1")]
    pub sequence: u32,
    #[prost(float, tag = "2")]
    pub move_x: f32,
    #[prost(float, tag = "3")]
    pub move_z: f32,
    #[prost(bool, tag = "4")]
    pub jump: bool,
    #[prost(float, tag = "5")]
    pub look_yaw: f32,
    #[prost(float, tag = "6")]
    pub look_pitch: f32,
}

#[derive(Clone, PartialEq, Message)]
pub struct PlayerState {
    #[prost(uint32, tag = "1")]
    pub player_id: u32,
    #[prost(uint32, tag = "2")]
    pub client_kind: u32,
    #[prost(fixed32, tag = "3")]
    pub color: u32,
    #[prost(float, tag = "4")]
    pub position_x: f32,
    #[prost(float, tag = "5")]
    pub position_z: f32,
    #[prost(float, tag = "6")]
    pub position_y: f32,
    #[prost(float, tag = "7")]
    pub look_yaw: f32,
    #[prost(float, tag = "8")]
    pub look_pitch: f32,
}

#[derive(Clone, PartialEq, Message)]
pub struct WorldSnapshot {
    #[prost(uint32, tag = "1")]
    pub tick: u32,
    #[prost(message, repeated, tag = "2")]
    pub players: Vec<PlayerState>,
}

impl From<&Player> for PlayerState {
    fn from(player: &Player) -> PlayerState {
        PlayerState {
            player_id: player.id,
            client_kind: u32::from(player.client_kind),
            color: player.color,
            position_x: player.position_x,
            position_z: player.position_z,
            position_y: player.position_y,
            look_yaw: player.look_yaw,
            look_pitch: player.look_pitch,
        }
    }
}

impl PlayerInput {
    pub fn as_fomoxa(&self) -> fomoxa::PlayerInput {
        fomoxa::PlayerInput {
            sequence: self.sequence,
            move_x: self.move_x,
            move_z: self.move_z,
            jump: self.jump,
            look_yaw: self.look_yaw,
            look_pitch: self.look_pitch,
        }
    }
}

pub fn snapshot_of(world: &World) -> WorldSnapshot {
    WorldSnapshot {
        tick: world.tick(),
        players: world
            .peers()
            .filter_map(|peer| world.player(peer))
            .map(PlayerState::from)
            .collect(),
    }
}

pub fn encode<M: Message>(message: &M) -> Vec<u8> {
    message.encode_to_vec()
}

pub fn decode<M: Message + Default>(payload: &[u8]) -> Result<M, prost::DecodeError> {
    M::decode(payload)
}
