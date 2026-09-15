class_name FomoxaExampleSession
extends RefCounted

enum Status {
	DISCONNECTED,
	CONNECTING,
	JOINING,
	JOINED,
	FAILED,
}

var status: Status = Status.DISCONNECTED
var failure_reason := ""
var local_player_id := 0
var plane_half_size := 10.0
var tick_rate := 30
var last_tick := -1
var snapshots_received := 0
var players := {}

var _connection: FomoxaConnection
var _display_name := ""
var _hello_sent := false
var _input_sequence := 0
var _next_input_msec := 0

func open(host: String, port: int, display_name: String) -> int:
	close()
	failure_reason = ""
	var schema := FomoxaExampleProtocol.build_schema()
	if schema == null:
		_fail("the generated schema could not be built")
		return ERR_INVALID_DATA
	var dialled := FomoxaTcp.connect_to_host(host, port, schema)
	if dialled[1] != OK:
		_fail("cannot reach %s:%d - %s" % [host, port, error_string(dialled[1])])
		return dialled[1]
	_connection = dialled[0]
	_display_name = display_name
	status = Status.CONNECTING
	return OK

func close() -> void:
	_release()
	status = Status.DISCONNECTED

func poll(now_msec: int) -> void:
	if _connection == null:
		return
	for event in _connection.poll(now_msec):
		match event.kind:
			FomoxaEvent.Kind.READY:
				status = Status.JOINING
			FomoxaEvent.Kind.MESSAGE:
				_on_message(event.message_id, event.payload)
			FomoxaEvent.Kind.HANDSHAKE_FAILED:
				_fail("handshake refused: %s" % event.reason)
				return
			FomoxaEvent.Kind.DISCONNECTED:
				_fail("disconnected: %s" % event.reason)
				return
	if status == Status.JOINING and not _hello_sent:
		var sent: int = _connection.send(FomoxaHandshake.CLIENT_HELLO_GAME_MESSAGE_ID, FomoxaExampleProtocol.encode_hello(_display_name))
		_hello_sent = sent == FomoxaConnection.Result.OK

func local_state() -> PlayerState:
	return players.get(local_player_id)

func update_input(now_msec: int, move: Vector2, jump: bool, look_yaw: float, look_pitch: float) -> void:
	if status != Status.JOINED or now_msec < _next_input_msec:
		return
	_input_sequence += 1
	var payload := FomoxaExampleProtocol.encode_input(_input_sequence, move, jump, look_yaw, look_pitch)
	var sent: int = _connection.send(FomoxaHandshake.PLAYER_INPUT_GAME_MESSAGE_ID, payload)
	if sent == FomoxaConnection.Result.OK:
		_next_input_msec = now_msec + 1000 / maxi(tick_rate, 1)

func _on_message(message_id: int, payload: PackedByteArray) -> void:
	if message_id == FomoxaHandshake.WELCOME_GAME_MESSAGE_ID:
		var welcome := FomoxaExampleProtocol.decode_welcome(payload)
		if welcome == null:
			return
		local_player_id = welcome.player_id
		plane_half_size = welcome.plane_half_size
		tick_rate = welcome.tick_rate
		status = Status.JOINED
	elif message_id == FomoxaHandshake.WORLD_SNAPSHOT_GAME_MESSAGE_ID:
		var snapshot := FomoxaExampleProtocol.decode_snapshot(payload)
		if snapshot == null:
			return
		last_tick = snapshot.tick
		snapshots_received += 1
		players.clear()
		for state: PlayerState in snapshot.players:
			players[state.player_id] = state

func _fail(reason: String) -> void:
	_release()
	failure_reason = reason
	status = Status.FAILED
	push_warning("fomoxa-example: %s" % reason)

func _release() -> void:
	if _connection != null:
		_connection.close()
		_connection.release()
		_connection = null
	_hello_sent = false
	_input_sequence = 0
	_next_input_msec = 0
	local_player_id = 0
	last_tick = -1
	snapshots_received = 0
	players.clear()
