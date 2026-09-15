class_name FomoxaEvent
extends RefCounted

enum Kind {
	CONNECTED,
	READY,
	HANDSHAKE_FAILED,
	MESSAGE,
	PROBE,
	ACK,
	DISCONNECTED,
}

const NO_VERDICT := -1

var kind: int = Kind.CONNECTED
var peer: int = 0
var message_id: int = 0
var payload := PackedByteArray()
var verdict: int = NO_VERDICT
var reason: String = ""

static func of(event_kind: int) -> FomoxaEvent:
	var event := FomoxaEvent.new()
	event.kind = event_kind
	return event

static func message(id: int, body: PackedByteArray) -> FomoxaEvent:
	var event := of(Kind.MESSAGE)
	event.message_id = id
	event.payload = body
	return event

static func handshake_failed(failure_verdict: int, why: String) -> FomoxaEvent:
	var event := of(Kind.HANDSHAKE_FAILED)
	event.verdict = failure_verdict
	event.reason = why
	return event

static func disconnected(why: String) -> FomoxaEvent:
	var event := of(Kind.DISCONNECTED)
	event.reason = why
	return event

static func kind_name(event_kind: int) -> String:
	match event_kind:
		Kind.CONNECTED:
			return "connected"
		Kind.READY:
			return "ready"
		Kind.HANDSHAKE_FAILED:
			return "handshake_failed"
		Kind.MESSAGE:
			return "message"
		Kind.PROBE:
			return "probe"
		Kind.ACK:
			return "ack"
		Kind.DISCONNECTED:
			return "disconnected"
		_:
			return "event %d" % event_kind

func _to_string() -> String:
	var text := kind_name(kind)
	if peer != 0:
		text += " peer=%d" % peer
	if kind == Kind.MESSAGE:
		text += " id=0x%08X bytes=%d" % [message_id, payload.size()]
	if verdict != NO_VERDICT:
		text += " verdict=%d" % verdict
	if reason != "":
		text += " (%s)" % reason
	return text
