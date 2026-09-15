use crate::generated::{
    ClientHelloGameCodec, DecodeError, PlayerInputGameCodec, Reader, WelcomeGameCodec,
    WorldSnapshotGameCodec, Writer,
};
use crate::models::protocol::{ClientHello, PlayerInput, Welcome, WorldSnapshot};

macro_rules! message_codec {
    ($encode:ident, $decode:ident, $model:ident, $codec:ident) => {
        pub fn $encode(value: &$model) -> Vec<u8> {
            let mut writer = Writer::new();
            $codec::encode(&mut writer, value);
            writer.into_bytes()
        }

        pub fn $decode(payload: &[u8]) -> Result<$model, DecodeError> {
            let mut value = $model::default();
            let mut reader = Reader::new(payload);
            $codec::decode(&mut reader, &mut value)?;
            Ok(value)
        }
    };
}

message_codec!(encode_client_hello, decode_client_hello, ClientHello, ClientHelloGameCodec);
message_codec!(encode_welcome, decode_welcome, Welcome, WelcomeGameCodec);
message_codec!(encode_player_input, decode_player_input, PlayerInput, PlayerInputGameCodec);
message_codec!(encode_world_snapshot, decode_world_snapshot, WorldSnapshot, WorldSnapshotGameCodec);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::protocol::PlayerState;

    const INPUT_BYTES_BEFORE_JUMP_AND_LOOK: usize = 12;

    #[test]
    fn world_snapshot_survives_a_round_trip() {
        let snapshot = WorldSnapshot {
            tick: 42,
            players: vec![
                PlayerState {
                    player_id: 1,
                    client_kind: 2,
                    color: 0xE6194BFF,
                    position_x: 1.5,
                    position_z: -3.25,
                    position_y: 0.75,
                    look_yaw: 1.25,
                    look_pitch: -0.5,
                },
                PlayerState {
                    player_id: 2,
                    client_kind: 0,
                    color: 0x3CB44BFF,
                    position_x: -9.0,
                    ..PlayerState::default()
                },
            ],
        };

        let decoded = decode_world_snapshot(&encode_world_snapshot(&snapshot)).unwrap();

        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn client_hello_survives_a_round_trip() {
        let hello = ClientHello {
            client_kind: 2,
            display_name: "người chơi".to_owned(),
        };

        assert_eq!(decode_client_hello(&encode_client_hello(&hello)).unwrap(), hello);
    }

    #[test]
    fn input_from_a_client_without_jump_and_look_decodes_with_defaults() {
        let current = PlayerInput {
            sequence: 7,
            move_x: 1.0,
            move_z: -0.5,
            jump: true,
            look_yaw: 2.0,
            look_pitch: 1.0,
        };
        let encoded = encode_player_input(&current);

        let decoded = decode_player_input(&encoded[..INPUT_BYTES_BEFORE_JUMP_AND_LOOK]).unwrap();

        assert_eq!(
            decoded,
            PlayerInput {
                sequence: 7,
                move_x: 1.0,
                move_z: -0.5,
                ..PlayerInput::default()
            }
        );
    }
}
