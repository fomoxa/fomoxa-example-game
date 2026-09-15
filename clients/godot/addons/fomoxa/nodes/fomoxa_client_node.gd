class_name FomoxaClientNode
extends Node

signal connected()
signal session_ready()
signal handshake_failed(verdict: int, reason: String)
signal message_received(message_id: int, payload: PackedByteArray)
signal probed()
signal acked()
signal disconnected(reason: String)

enum Network {
	TCP,
	UDP,
}

@export var host: String = "127.0.0.1"
@export var port: int = 7788
@export var network: Network = Network.TCP
@export var autostart: bool = false

var schema: FomoxaSchema
var config: FomoxaConfig
var connection: FomoxaConnection

func _ready() -> void:
	if autostart:
		start()

func start() -> int:
	if connection != null:
		return OK
	if schema == null:
		push_error("fomoxa: set %s.schema before starting it" % name)
		return ERR_UNCONFIGURED

	var made := FomoxaUdp.connect_to_host(host, port, schema, config) if network == Network.UDP else FomoxaTcp.connect_to_host(host, port, schema, config)
	if made[1] != OK:
		push_error("fomoxa: %s could not reach %s:%d - %s" % [name, host, port, error_string(made[1])])
		return made[1]

	connection = made[0]
	return OK

func stop() -> void:
	if connection == null:
		return
	connection.close()
	connection.release()
	connection = null

func send(message_id: int, payload: PackedByteArray) -> int:
	if connection == null:
		return FomoxaConnection.Result.CLOSED
	return connection.send(message_id, payload)

func ready_to_send() -> bool:
	return connection != null and connection.ready()

func poll() -> void:
	if connection == null:
		return
	for event in connection.poll(Time.get_ticks_msec()):
		_emit(event)

func _process(_delta: float) -> void:
	poll()

func _exit_tree() -> void:
	stop()

func _emit(event: FomoxaEvent) -> void:
	match event.kind:
		FomoxaEvent.Kind.CONNECTED:
			connected.emit()
		FomoxaEvent.Kind.READY:
			session_ready.emit()
		FomoxaEvent.Kind.HANDSHAKE_FAILED:
			handshake_failed.emit(event.verdict, event.reason)
		FomoxaEvent.Kind.MESSAGE:
			message_received.emit(event.message_id, event.payload)
		FomoxaEvent.Kind.PROBE:
			probed.emit()
		FomoxaEvent.Kind.ACK:
			acked.emit()
		FomoxaEvent.Kind.DISCONNECTED:
			disconnected.emit(event.reason)
