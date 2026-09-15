class_name FomoxaServer
extends RefCounted

var _listener: FomoxaListener
var _schema: FomoxaSchema
var _config: FomoxaConfig

var _peers: Dictionary = {}
var _retiring: Array = []
var _next_peer_id: int = 1

static func over(listener: FomoxaListener, schema: FomoxaSchema, config: FomoxaConfig = null) -> FomoxaServer:
	return FomoxaServer.new(listener, schema, config)

func _init(listener: FomoxaListener, schema: FomoxaSchema, config: FomoxaConfig) -> void:
	_listener = listener
	_schema = schema
	_config = FomoxaConfig.new() if config == null else config.normalized()

func local_port() -> int:
	return _listener.local_port()

func peers() -> Array:
	return _peers.keys()

func peer_state(id: int) -> int:
	var connection: FomoxaConnection = _peers.get(id)
	if connection == null:
		return FomoxaSession.State.CLOSED
	return connection.state()

func poll(now_msec: int) -> Array:
	var events: Array = []

	for id in _retiring:
		var leaving: FomoxaConnection = _peers.get(id)
		if leaving != null:
			leaving.poll(now_msec)
			leaving.release()
			_peers.erase(id)
	_retiring = []

	for transport in _listener.accept(_config.max_accepts_per_poll):
		var id := _next_peer_id
		_next_peer_id += 1
		_peers[id] = FomoxaConnection.peer(transport, _schema, _config, id)

	for id in _peers.keys():
		var connection: FomoxaConnection = _peers[id]
		events.append_array(connection.poll(now_msec))
		if connection.finished():
			_retiring.append(id)

	return events

func send(id: int, message_id: int, payload: PackedByteArray) -> int:
	var connection: FomoxaConnection = _peers.get(id)
	if connection == null:
		return FomoxaConnection.Result.CLOSED
	return connection.send(message_id, payload)

func broadcast(message_id: int, payload: PackedByteArray) -> void:
	for id in _peers:
		var connection: FomoxaConnection = _peers[id]
		if connection.ready():
			connection.send(message_id, payload)

func disconnect_peer(id: int) -> void:
	var connection: FomoxaConnection = _peers.get(id)
	if connection == null:
		return
	connection.close()
	connection.release()
	_peers.erase(id)

func close() -> void:
	for id in _peers:
		var connection: FomoxaConnection = _peers[id]
		connection.close()
		connection.release()
	_peers.clear()
	_retiring = []
	_listener.close()
