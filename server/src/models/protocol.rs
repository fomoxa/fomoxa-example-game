use fomoxa_attributes::*;

#[network]
#[codec(game)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClientHello {
    #[network(u8)]
    #[codec(game)]
    pub client_kind: u8,

    #[network(string)]
    #[codec(game)]
    pub display_name: String,
}

#[network]
#[codec(game)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Welcome {
    #[network(u32)]
    #[codec(game)]
    pub player_id: u32,

    #[network(f32)]
    #[codec(game)]
    pub plane_half_size: f32,

    #[network(u16)]
    #[codec(game)]
    pub tick_rate: u16,
}

#[network]
#[codec(game)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PlayerInput {
    #[network(u32)]
    #[codec(game)]
    pub sequence: u32,

    #[network(f32)]
    #[codec(game)]
    pub move_x: f32,

    #[network(f32)]
    #[codec(game)]
    pub move_z: f32,

    #[network(bool)]
    #[codec(game)]
    pub jump: bool,

    #[network(f32)]
    #[codec(game)]
    pub look_yaw: f32,

    #[network(f32)]
    #[codec(game)]
    pub look_pitch: f32,
}

#[network]
#[codec(game)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PlayerState {
    #[network(u32)]
    #[codec(game)]
    pub player_id: u32,

    #[network(u8)]
    #[codec(game)]
    pub client_kind: u8,

    #[network(u32)]
    #[codec(game)]
    pub color: u32,

    #[network(f32)]
    #[codec(game)]
    pub position_x: f32,

    #[network(f32)]
    #[codec(game)]
    pub position_z: f32,

    #[network(f32)]
    #[codec(game)]
    pub position_y: f32,

    #[network(f32)]
    #[codec(game)]
    pub look_yaw: f32,

    #[network(f32)]
    #[codec(game)]
    pub look_pitch: f32,
}

#[network]
#[codec(game)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct WorldSnapshot {
    #[network(u32)]
    #[codec(game)]
    pub tick: u32,

    #[network(Array<PlayerState>)]
    #[codec(game)]
    pub players: Vec<PlayerState>,
}
