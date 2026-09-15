class_name FomoxaServerNode
extends Node

signal peer_connected(peer: int)
signal peer_ready(peer: int)
signal peer_handshake_failed(peer: int, verdict: int, reason: String)
signal message_received(peer: int, message_id: int, payload: PackedByteArray)
signal peer_probed(peer: int)
signal peer_acked(peer: int)
signal peer_disconnected(peer: int, reason: String)

enum Network {
	TCP,
	UDP,
}

@export var port: int = 7788
@export var bind_address: String = "*"
@export var network: Network = Network.TCP
@export var autostart: bool = false

var schema: FomoxaSchema
var config: FomoxaConfig
var server: FomoxaServer

func _ready() -> void:
	if autostart:
		start()

func start() -> int:
	if server != null:
		return OK
	if schema == null:
		push_error("fomoxa: set %s.schema before starting it" % name)
		return ERR_UNCONFIGURED

	var made := FomoxaUdp.listen(port, schema, config, bind_address) if network == Network.UDP else FomoxaTcp.listen(port, schema, config, bind_address)
	if made[1] != OK:
		push_error("fomoxa: %s could not listen on port %d - %s" % [name, port, error_string(made[1])])
		return made[1]

	server = made[0]
	return OK

func stop() -> void:
	if server == null:
		return
	server.close()
	server = null

func listening() -> bool:
	return server != null

func local_port() -> int:
	return 0 if server == null else server.local_port()

func send(peer: int, message_id: int, payload: PackedByteArray) -> int:
	if server == null:
		return FomoxaConnection.Result.CLOSED
	return server.send(peer, message_id, payload)

func broadcast(message_id: int, payload: PackedByteArray) -> void:
	if server != null:
		server.broadcast(message_id, payload)

func poll() -> void:
	if server == null:
		return
	for event in server.poll(Time.get_ticks_msec()):
		_emit(event)

func _process(_delta: float) -> void:
	poll()

func _exit_tree() -> void:
	stop()

func _emit(event: FomoxaEvent) -> void:
	match event.kind:
		FomoxaEvent.Kind.CONNECTED:
			peer_connected.emit(event.peer)
		FomoxaEvent.Kind.READY:
			peer_ready.emit(event.peer)
		FomoxaEvent.Kind.HANDSHAKE_FAILED:
			peer_handshake_failed.emit(event.peer, event.verdict, event.reason)
		FomoxaEvent.Kind.MESSAGE:
			message_received.emit(event.peer, event.message_id, event.payload)
		FomoxaEvent.Kind.PROBE:
			peer_probed.emit(event.peer)
		FomoxaEvent.Kind.ACK:
			peer_acked.emit(event.peer)
		FomoxaEvent.Kind.DISCONNECTED:
			peer_disconnected.emit(event.peer, event.reason)
