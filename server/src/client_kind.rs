pub const BOT: u8 = 0;
pub const UNITY: u8 = 1;
pub const GODOT: u8 = 2;
pub const UNREAL: u8 = 3;
pub const KAIJU: u8 = 4;

pub fn name(kind: u8) -> &'static str {
    match kind {
        BOT => "bot",
        UNITY => "unity",
        GODOT => "godot",
        UNREAL => "unreal",
        KAIJU => "kaiju",
        _ => "unknown",
    }
}
