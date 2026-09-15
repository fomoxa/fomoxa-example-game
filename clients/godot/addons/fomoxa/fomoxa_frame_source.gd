class_name FomoxaFrameSource
extends RefCounted

enum Result {
	FRAME,
	EMPTY,
	DROPPED,
	CLOSED,
	FAILED,
}

var _transport: FomoxaTransport
var _stream: bool
var _decoder: FomoxaFrame.Decoder
var _capacity: int
var _failure: String = ""

func _init(transport: FomoxaTransport, capacity: int) -> void:
	_transport = transport
	_capacity = capacity
	_stream = transport.kind() == FomoxaTransport.Kind.STREAM
	if _stream:
		_decoder = FomoxaFrame.Decoder.new()

func failure() -> String:
	return _failure

func next() -> Array:
	if _stream:
		return _next_from_stream()
	return _next_from_packet()

func _next_from_stream() -> Array:
	while true:
		var taken := _decoder.next()
		if taken[1] != "":
			_failure = taken[1]
			return [Result.FAILED, null]
		if taken[0] != null:
			return [Result.FRAME, taken[0]]

		var read := _transport.receive(_capacity)
		match read[0]:
			FomoxaTransport.Status.OK:
				var bytes: PackedByteArray = read[1]
				if bytes.is_empty():
					return [Result.EMPTY, null]
				_decoder.feed(bytes)
			FomoxaTransport.Status.PENDING:
				return [Result.EMPTY, null]
			FomoxaTransport.Status.TOO_SMALL:
				_capacity = maxi(read[2], _capacity * 2)
			FomoxaTransport.Status.CLOSED:
				return [Result.CLOSED, null]
			_:
				_failure = _transport_failure("the transport reported an error while reading")
				return [Result.FAILED, null]
	return [Result.EMPTY, null]

func _next_from_packet() -> Array:
	while true:
		var read := _transport.receive(_capacity)
		match read[0]:
			FomoxaTransport.Status.OK:
				var decoded := FomoxaFrame.decode_packet(read[1])
				if decoded[1] != "":
					return [Result.DROPPED, null]
				return [Result.FRAME, decoded[0]]
			FomoxaTransport.Status.PENDING:
				return [Result.EMPTY, null]
			FomoxaTransport.Status.TOO_SMALL:
				_capacity = maxi(read[2], _capacity * 2)
			FomoxaTransport.Status.CLOSED:
				return [Result.CLOSED, null]
			_:
				_failure = _transport_failure("the transport reported an error while reading")
				return [Result.FAILED, null]
	return [Result.EMPTY, null]

func _transport_failure(fallback: String) -> String:
	var reported := _transport.failure()
	if reported != "":
		return reported
	return fallback
