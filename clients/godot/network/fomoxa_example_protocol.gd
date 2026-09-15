class_name FomoxaExampleProtocol
extends RefCounted

const DEFAULT_HOST := "127.0.0.1"
const DEFAULT_PORT := 9321
const CLIENT_KIND_GODOT := 2
const MAX_LOOK_PITCH := 1.5
const CLIENT_KIND_NAMES := {
	0: "Bot",
	1: "Unity",
	2: "Godot",
	3: "Unreal",
	4: "Kaiju",
	5: "nunuStudio",
}

static func build_schema() -> FomoxaSchema:
	var made := FomoxaSchema.from_generated(FomoxaHandshake.SCHEMA_FINGERPRINT, FomoxaHandshake.MESSAGES)
	if made[1] != "":
		push_error("fomoxa: %s" % made[1])
		return null
	return made[0]

static func encode_hello(display_name: String) -> PackedByteArray:
	var hello := ClientHello.new()
	hello.client_kind = CLIENT_KIND_GODOT
	hello.display_name = display_name
	var writer := FomoxaRuntime.Writer.new()
	ClientHelloGameCodec.encode(writer, hello)
	return writer.bytes()

static func encode_input(sequence: int, move: Vector2, jump: bool, look_yaw: float, look_pitch: float) -> PackedByteArray:
	var input := PlayerInput.new()
	input.sequence = sequence
	input.move_x = move.x
	input.move_z = move.y
	input.jump = jump
	input.look_yaw = look_yaw
	input.look_pitch = look_pitch
	var writer := FomoxaRuntime.Writer.new()
	PlayerInputGameCodec.encode(writer, input)
	return writer.bytes()

static func decode_welcome(payload: PackedByteArray) -> Welcome:
	var welcome := Welcome.new()
	var error := WelcomeGameCodec.decode(FomoxaRuntime.Reader.new(payload), welcome)
	if error != null:
		push_error("fomoxa: undecodable Welcome: %s" % error.message())
		return null
	return welcome

static func decode_snapshot(payload: PackedByteArray) -> WorldSnapshot:
	var snapshot := WorldSnapshot.new()
	var error := WorldSnapshotGameCodec.decode(FomoxaRuntime.Reader.new(payload), snapshot)
	if error != null:
		push_error("fomoxa: undecodable WorldSnapshot: %s" % error.message())
		return null
	return snapshot

static func kind_name(client_kind: int) -> String:
	return CLIENT_KIND_NAMES.get(client_kind, "Unknown")

static func player_label(state: PlayerState, local_player_id: int) -> String:
	var suffix := " (you)" if state.player_id == local_player_id else ""
	return "%s #%d%s" % [kind_name(state.client_kind), state.player_id, suffix]

static func read_options(defaults: Dictionary) -> Dictionary:
	var options := defaults.duplicate()
	for argument: String in OS.get_cmdline_user_args():
		var parts: PackedStringArray = argument.trim_prefix("--").split("=", true, 1)
		if parts.size() != 2 or not options.has(parts[0]):
			continue
		match typeof(options[parts[0]]):
			TYPE_INT:
				options[parts[0]] = parts[1].to_int()
			TYPE_FLOAT:
				options[parts[0]] = parts[1].to_float()
			_:
				options[parts[0]] = parts[1]
	return options
