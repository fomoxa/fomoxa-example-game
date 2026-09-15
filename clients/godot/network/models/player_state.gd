# fomoxa:model codec=game
class_name PlayerState

# fomoxa:u32 codec=game
var player_id: int = 0

# fomoxa:u8 codec=game
var client_kind: int = 0

# fomoxa:u32 codec=game
var color: int = 0

# fomoxa:f32 codec=game
var position_x: float = 0.0

# fomoxa:f32 codec=game
var position_z: float = 0.0

# fomoxa:f32 codec=game
var position_y: float = 0.0

# fomoxa:f32 codec=game
var look_yaw: float = 0.0

# fomoxa:f32 codec=game
var look_pitch: float = 0.0
