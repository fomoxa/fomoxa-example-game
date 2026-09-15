class_name FomoxaTcpTransport
extends FomoxaTransport

var _peer: StreamPeerTCP
var _backlog := PackedByteArray()
var _connected := false
var _closed := false
var _failure := ""

static func over(peer: StreamPeerTCP) -> FomoxaTcpTransport:
	var transport := FomoxaTcpTransport.new()
	transport._peer = peer
	peer.set_no_delay(true)
	return transport

static func connect_to(host: String, port: int) -> Array:
	var peer := StreamPeerTCP.new()
	var error := peer.connect_to_host(host, port)
	if error != OK:
		return [null, error]
	return [over(peer), OK]

func kind() -> int:
	return Kind.STREAM

func failure() -> String:
	return _failure

func send(bytes: PackedByteArray) -> int:
	if _closed:
		return Status.CLOSED

	var live := _pump()
	if live != Status.OK:
		return live
	if not _connected:
		return Status.PENDING

	if not _backlog.is_empty():
		var pushed := _push_backlog()
		if pushed != Status.OK:
			return pushed
		if not _backlog.is_empty():
			return Status.PENDING

	var result := _peer.put_partial_data(bytes)
	if result[0] != OK:
		return _die("the socket refused a write: %s" % error_string(result[0]))

	var sent: int = result[1]
	if sent == 0:
		return Status.PENDING
	if sent < bytes.size():
		_backlog = bytes.slice(sent)
	return Status.OK

func receive(capacity: int) -> Array:
	if _closed:
		return [Status.CLOSED, PackedByteArray(), 0]

	var live := _pump()
	if live != Status.OK:
		return [live, PackedByteArray(), 0]
	if not _connected:
		return [Status.PENDING, PackedByteArray(), 0]

	var available := _peer.get_available_bytes()
	if available <= 0:
		return [Status.PENDING, PackedByteArray(), 0]

	var result := _peer.get_partial_data(mini(available, capacity))
	if result[0] != OK:
		return [_die("the socket refused a read: %s" % error_string(result[0])), PackedByteArray(), 0]
	return [Status.OK, result[1], 0]

func close_send() -> void:
	pass

func close() -> void:
	if _closed:
		return
	_closed = true
	_peer.disconnect_from_host()

func _pump() -> int:
	_peer.poll()
	match _peer.get_status():
		StreamPeerTCP.STATUS_CONNECTED:
			_connected = true
			return Status.OK
		StreamPeerTCP.STATUS_CONNECTING:
			return Status.OK
		StreamPeerTCP.STATUS_NONE:
			if _connected:
				_closed = true
				return Status.CLOSED
			return _die("the connection was never established")
		_:
			return _die("the socket reported an error")

func _push_backlog() -> int:
	var result := _peer.put_partial_data(_backlog)
	if result[0] != OK:
		return _die("the socket refused a write: %s" % error_string(result[0]))
	var sent: int = result[1]
	if sent > 0:
		_backlog = _backlog.slice(sent)
	return Status.OK

func _die(reason: String) -> int:
	_failure = reason
	_closed = true
	return Status.ERROR
