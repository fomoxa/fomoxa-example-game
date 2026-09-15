class_name FomoxaUdpTransport
extends FomoxaTransport

const MAX_DATAGRAM := 65507

var _peer: PacketPeerUDP
var _held := PackedByteArray()
var _holding := false
var _closed := false
var _failure := ""

static func over(peer: PacketPeerUDP) -> FomoxaUdpTransport:
	var transport := FomoxaUdpTransport.new()
	transport._peer = peer
	return transport

static func connect_to(host: String, port: int) -> Array:
	var peer := PacketPeerUDP.new()
	var error := peer.connect_to_host(host, port)
	if error != OK:
		return [null, error]
	return [over(peer), OK]

func kind() -> int:
	return Kind.PACKET

func failure() -> String:
	return _failure

func send(bytes: PackedByteArray) -> int:
	if _closed:
		return Status.CLOSED
	if bytes.size() > MAX_DATAGRAM:
		return Status.TOO_LARGE

	var error := _peer.put_packet(bytes)
	match error:
		OK:
			return Status.OK
		ERR_BUSY, ERR_UNAVAILABLE:
			return Status.PENDING
		_:
			_failure = "the socket refused a datagram: %s" % error_string(error)
			_closed = true
			return Status.ERROR

func receive(capacity: int) -> Array:
	if _holding:
		if _held.size() > capacity:
			return [Status.TOO_SMALL, PackedByteArray(), _held.size()]
		var kept := _held
		_held = PackedByteArray()
		_holding = false
		return [Status.OK, kept, 0]

	if _closed:
		return [Status.CLOSED, PackedByteArray(), 0]
	if _peer.get_available_packet_count() <= 0:
		return [Status.PENDING, PackedByteArray(), 0]

	var packet := _peer.get_packet()
	var error := _peer.get_packet_error()
	if error != OK:
		_failure = "the socket refused a datagram: %s" % error_string(error)
		_closed = true
		return [Status.ERROR, PackedByteArray(), 0]

	if packet.size() > capacity:
		_held = packet
		_holding = true
		return [Status.TOO_SMALL, PackedByteArray(), packet.size()]
	return [Status.OK, packet, 0]

func close_send() -> void:
	pass

func close() -> void:
	if _closed:
		return
	_closed = true
	_peer.close()
