class_name FomoxaTransport
extends RefCounted

enum Kind {
	STREAM,
	PACKET,
}

enum Status {
	OK,
	PENDING,
	CLOSED,
	ERROR,
	TOO_LARGE,
	TOO_SMALL,
}

static func status_name(status: int) -> String:
	match status:
		Status.OK:
			return "ok"
		Status.PENDING:
			return "pending"
		Status.CLOSED:
			return "closed"
		Status.ERROR:
			return "error"
		Status.TOO_LARGE:
			return "too large"
		Status.TOO_SMALL:
			return "too small"
		_:
			return "status %d" % status

func kind() -> int:
	push_error("fomoxa: a transport must say whether it is a stream or a packet transport")
	return Kind.STREAM

func send(_bytes: PackedByteArray) -> int:
	push_error("fomoxa: a transport must implement send()")
	return Status.ERROR

func receive(_capacity: int) -> Array:
	push_error("fomoxa: a transport must implement receive()")
	return [Status.ERROR, PackedByteArray(), 0]

func close_send() -> void:
	pass

func close() -> void:
	pass

func failure() -> String:
	return ""
