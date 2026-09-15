extends Node3D

const BODY_SIZE := Vector3(0.8, 1.2, 0.8)
const HEAD_SIZE := Vector3(0.55, 0.45, 0.55)
const VISOR_SIZE := Vector3(0.4, 0.12, 0.08)
const HEAD_CENTER_HEIGHT := 1.45
const EYE_HEIGHT := 1.5
const LABEL_HEIGHT := 2.1
const OVERVIEW_POSITION := Vector3(0.0, 16.0, 13.0)
const MOUSE_SENSITIVITY := 0.0025
const VISOR_COLOR := Color(0.05, 0.05, 0.08)
const HEAD_NODE_NAME := "Head"
const LABEL_NODE_NAME := "Label"

var session := FomoxaExampleSession.new()
var _options := {}
var _avatars := {}
var _look_yaw := 0.0
var _look_pitch := 0.0

@onready var _floor: Node3D = $Floor
@onready var _camera: Camera3D = $Camera
@onready var _status_label: Label = $Hud/Status

func _ready() -> void:
	_options = FomoxaExampleProtocol.read_options({
		"host": FomoxaExampleProtocol.DEFAULT_HOST,
		"port": FomoxaExampleProtocol.DEFAULT_PORT,
		"name": "godot-%d" % (randi() % 1000),
	})
	_connect()

func _process(_delta: float) -> void:
	var now := Time.get_ticks_msec()
	session.poll(now)
	session.update_input(now, _desired_move(), Input.is_physical_key_pressed(KEY_SPACE), _look_yaw, _look_pitch)
	if not is_equal_approx(_floor.half_size, session.plane_half_size):
		_floor.half_size = session.plane_half_size
	_sync_avatars()
	_place_camera()
	_status_label.text = _status_text()

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	elif event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		_look_yaw = wrapf(_look_yaw - event.relative.x * MOUSE_SENSITIVITY, -PI, PI)
		_look_pitch = clampf(_look_pitch - event.relative.y * MOUSE_SENSITIVITY, -FomoxaExampleProtocol.MAX_LOOK_PITCH, FomoxaExampleProtocol.MAX_LOOK_PITCH)
	elif event is InputEventKey and event.pressed and not event.echo:
		if event.physical_keycode == KEY_ESCAPE:
			Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
		elif event.physical_keycode == KEY_R and session.status == FomoxaExampleSession.Status.FAILED:
			_connect()

func _exit_tree() -> void:
	session.close()

func _connect() -> void:
	session.open(_options.host, _options.port, _options.name)

func _desired_move() -> Vector2:
	var strafe := _axis(KEY_A, KEY_LEFT, KEY_D, KEY_RIGHT)
	var forward := _axis(KEY_S, KEY_DOWN, KEY_W, KEY_UP)
	var right := Vector2(cos(_look_yaw), -sin(_look_yaw))
	var ahead := Vector2(-sin(_look_yaw), -cos(_look_yaw))
	return right * strafe + ahead * forward

func _axis(negative: Key, negative_alternate: Key, positive: Key, positive_alternate: Key) -> float:
	var value := 0.0
	if Input.is_physical_key_pressed(negative) or Input.is_physical_key_pressed(negative_alternate):
		value -= 1.0
	if Input.is_physical_key_pressed(positive) or Input.is_physical_key_pressed(positive_alternate):
		value += 1.0
	return value

func _sync_avatars() -> void:
	for player_id in _avatars.keys():
		if not session.players.has(player_id):
			_avatars[player_id].queue_free()
			_avatars.erase(player_id)
	for player_id in session.players:
		var state: PlayerState = session.players[player_id]
		if not _avatars.has(player_id):
			_avatars[player_id] = _create_avatar(state)
		var avatar: Node3D = _avatars[player_id]
		avatar.visible = state.player_id != session.local_player_id
		avatar.position = Vector3(state.position_x, state.position_y, state.position_z)
		avatar.rotation.y = state.look_yaw
		var head: Node3D = avatar.get_node(HEAD_NODE_NAME)
		head.rotation.x = state.look_pitch
		var label: Label3D = avatar.get_node(LABEL_NODE_NAME)
		label.text = FomoxaExampleProtocol.player_label(state, session.local_player_id)

func _create_avatar(state: PlayerState) -> Node3D:
	var avatar := Node3D.new()
	var body_material := _flat_material(Color.hex(state.color))
	avatar.add_child(_box(BODY_SIZE, Vector3(0.0, BODY_SIZE.y * 0.5, 0.0), body_material))

	var head := Node3D.new()
	head.name = HEAD_NODE_NAME
	head.position = Vector3(0.0, HEAD_CENTER_HEIGHT, 0.0)
	head.add_child(_box(HEAD_SIZE, Vector3.ZERO, body_material))
	var visor_position := Vector3(0.0, 0.05, -(HEAD_SIZE.z + VISOR_SIZE.z) * 0.5)
	head.add_child(_box(VISOR_SIZE, visor_position, _flat_material(VISOR_COLOR)))
	avatar.add_child(head)

	var label := Label3D.new()
	label.name = LABEL_NODE_NAME
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.position = Vector3(0.0, LABEL_HEIGHT, 0.0)
	label.font_size = 48
	label.outline_size = 12
	avatar.add_child(label)

	add_child(avatar)
	return avatar

func _place_camera() -> void:
	var own := session.local_state()
	if own == null:
		_camera.look_at_from_position(OVERVIEW_POSITION, Vector3.ZERO, Vector3.UP)
		return
	_camera.position = Vector3(own.position_x, own.position_y + EYE_HEIGHT, own.position_z)
	_camera.rotation = Vector3(_look_pitch, _look_yaw, 0.0)

func _box(size: Vector3, offset: Vector3, material: Material) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	var mesh := BoxMesh.new()
	mesh.size = size
	instance.mesh = mesh
	instance.position = offset
	instance.material_override = material
	return instance

func _flat_material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	return material

func _status_text() -> String:
	match session.status:
		FomoxaExampleSession.Status.CONNECTING:
			return "Connecting to %s:%d..." % [_options.host, _options.port]
		FomoxaExampleSession.Status.JOINING:
			return "Handshake accepted, joining..."
		FomoxaExampleSession.Status.JOINED:
			return "Player #%d · %d players · tick %d\nclick: capture mouse · WASD: move · Space: jump · Esc: release mouse" % [session.local_player_id, session.players.size(), session.last_tick]
		FomoxaExampleSession.Status.FAILED:
			return "%s · press R to reconnect" % session.failure_reason
	return "Disconnected"
