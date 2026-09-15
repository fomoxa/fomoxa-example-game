class_name FomoxaSession
extends RefCounted

enum Role {
	CLIENT,
	SERVER,
}

enum State {
	HANDSHAKING,
	READY,
	CLOSED,
}

class Outcome:
	extends RefCounted

	var frame := PackedByteArray()
	var event: FomoxaEvent = null

	func with_frame(bytes: PackedByteArray) -> Outcome:
		frame = bytes
		return self

	func with_event(raised: FomoxaEvent) -> Outcome:
		event = raised
		return self

var role: int = Role.CLIENT
var state: int = State.HANDSHAKING

var _schema: FomoxaSchema
var _config: FomoxaConfig

var _started_at: int = 0
var _last_activity: int = 0
var _probe_sent_at: int = -1
var _terminal := false

var _query_seen := false
var _hello_seen := false
var _pending_query: Array = []

func _init(session_role: int, schema: FomoxaSchema, config: FomoxaConfig) -> void:
	role = session_role
	_schema = schema
	_config = config

func closed() -> bool:
	return state == State.CLOSED

func terminal_reported() -> bool:
	return _terminal

func start(now_msec: int) -> Outcome:
	_started_at = now_msec
	_last_activity = now_msec
	if role == Role.CLIENT:
		return Outcome.new().with_frame(FomoxaFrame.encode_handshake(FomoxaNegotiation.encode_hello(_schema)))
	return Outcome.new()

func handle_frame(frame: FomoxaFrame, now_msec: int) -> Outcome:
	if state == State.CLOSED:
		return Outcome.new()

	_last_activity = now_msec
	_probe_sent_at = -1

	match frame.type:
		FomoxaFrame.Type.PROBE:
			var outcome := Outcome.new().with_frame(FomoxaFrame.encode_ack())
			if state == State.READY:
				outcome.with_event(FomoxaEvent.of(FomoxaEvent.Kind.PROBE))
			return outcome

		FomoxaFrame.Type.ACK:
			if state == State.READY:
				return Outcome.new().with_event(FomoxaEvent.of(FomoxaEvent.Kind.ACK))
			return Outcome.new()

		FomoxaFrame.Type.DATA:
			if state == State.READY:
				return Outcome.new().with_event(FomoxaEvent.message(frame.message_id, frame.payload))
			return Outcome.new()

		_:
			if state != State.HANDSHAKING:
				return Outcome.new()
			if role == Role.CLIENT:
				return _read_as_client(frame.payload)
			return _read_as_server(frame.payload)

func tick(now_msec: int) -> Outcome:
	if state == State.CLOSED:
		return Outcome.new()

	if role == Role.CLIENT and state == State.HANDSHAKING:
		if now_msec - _started_at >= _config.handshake_timeout_msec:
			return _fail(FomoxaEvent.NO_VERDICT, "the handshake did not finish within %d ms" % _config.handshake_timeout_msec)
		return Outcome.new()

	if _probe_sent_at >= 0:
		if now_msec - _probe_sent_at >= _config.heartbeat_timeout_msec:
			return _die("the peer did not answer a probe within %d ms" % _config.heartbeat_timeout_msec)
		return Outcome.new()

	if now_msec - _last_activity >= _silence_window():
		_probe_sent_at = now_msec
		return Outcome.new().with_frame(FomoxaFrame.encode_probe())

	return Outcome.new()

func transport_closed(reason: String) -> Outcome:
	return _die(reason)

func close() -> void:
	state = State.CLOSED
	_terminal = true

func _silence_window() -> int:
	if role == Role.SERVER and state == State.HANDSHAKING:
		return _config.handshake_timeout_msec
	return _config.heartbeat_interval_msec

func _read_as_client(payload: PackedByteArray) -> Outcome:
	if payload.is_empty():
		return _fail(FomoxaNegotiation.Verdict.MALFORMED, "the server sent an empty handshake payload")

	if payload[0] == FomoxaNegotiation.QUERY:
		return _answer_query(payload)

	if payload.size() != 1:
		return _fail(FomoxaNegotiation.Verdict.MALFORMED, "a verdict must be one byte, this one is %d" % payload.size())
	var verdict := payload[0]
	if verdict > FomoxaNegotiation.Verdict.MALFORMED:
		return _fail(FomoxaNegotiation.Verdict.MALFORMED, "the server sent verdict %d, which is not one of 0-3" % verdict)
	if verdict == FomoxaNegotiation.Verdict.ACCEPT:
		state = State.READY
		return Outcome.new().with_event(FomoxaEvent.of(FomoxaEvent.Kind.READY))
	return _fail(verdict, FomoxaNegotiation.verdict_name(verdict))

func _answer_query(payload: PackedByteArray) -> Outcome:
	if _query_seen:
		return _fail(FomoxaNegotiation.Verdict.MALFORMED, "the server asked a second time; a handshake is never more than two rounds")
	_query_seen = true

	var decoded := FomoxaNegotiation.decode_query(payload)
	if decoded[1] != "":
		return _fail(FomoxaNegotiation.Verdict.MALFORMED, decoded[1])

	var answers := []
	for item in decoded[0]:
		var known := _schema.message(item.id)
		if known == null:
			return _fail(FomoxaNegotiation.Verdict.MALFORMED, "the server asked about message 0x%08X, which was never in the hello" % item.id)
		if item.field_count < 1 or item.field_count >= known.field_count():
			return _fail(FomoxaNegotiation.Verdict.MALFORMED, "the server asked message 0x%08X for field %d of %d" % [item.id, item.field_count, known.field_count()])
		answers.append(FomoxaNegotiation.ReplyItem.new(item.id, known.prefixes[item.field_count - 1]))

	return Outcome.new().with_frame(FomoxaFrame.encode_handshake(FomoxaNegotiation.encode_reply(answers)))

func _read_as_server(payload: PackedByteArray) -> Outcome:
	if not _hello_seen:
		_hello_seen = true
		return _judge_hello(payload)
	if _pending_query.is_empty():
		return _refuse(FomoxaNegotiation.Verdict.MALFORMED, "a third handshake payload arrived while still handshaking")
	return _judge_query_reply(payload)

func _judge_hello(payload: PackedByteArray) -> Outcome:
	var decoded := FomoxaNegotiation.decode_hello(payload)
	if decoded[1] != "":
		return _refuse(FomoxaNegotiation.Verdict.MALFORMED, decoded[1])

	var hello: FomoxaNegotiation.Hello = decoded[0]
	if hello.version != FomoxaNegotiation.PROTOCOL_VERSION:
		return _refuse(FomoxaNegotiation.Verdict.VERSION, "the peer speaks handshake version %d, this one speaks %d" % [hello.version, FomoxaNegotiation.PROTOCOL_VERSION])

	if hello.schema_fingerprint == _schema.fingerprint:
		return _accept()

	var asks := []
	for item in hello.items:
		var outcome := _schema.check_message(item.id, item.field_count, item.fingerprint)
		match outcome[0]:
			FomoxaSchema.Check.REJECT:
				return _refuse(FomoxaNegotiation.Verdict.CONFLICT, "message 0x%08X puts different fields at an index both ends carry" % item.id)
			FomoxaSchema.Check.NEED_PREFIX:
				asks.append(FomoxaNegotiation.QueryItem.new(item.id, outcome[1]))

	if asks.is_empty():
		return _accept()

	_pending_query = asks
	return Outcome.new().with_frame(FomoxaFrame.encode_handshake(FomoxaNegotiation.encode_query(asks)))

func _judge_query_reply(payload: PackedByteArray) -> Outcome:
	var decoded := FomoxaNegotiation.decode_reply(payload)
	if decoded[1] != "":
		return _refuse(FomoxaNegotiation.Verdict.MALFORMED, decoded[1])

	var answers: Array = decoded[0]
	if answers.size() != _pending_query.size():
		return _refuse(FomoxaNegotiation.Verdict.MALFORMED, "the peer answered %d of the %d messages it was asked about" % [answers.size(), _pending_query.size()])

	for i in _pending_query.size():
		var asked: FomoxaNegotiation.QueryItem = _pending_query[i]
		var answer: FomoxaNegotiation.ReplyItem = answers[i]
		if answer.id != asked.id:
			return _refuse(FomoxaNegotiation.Verdict.MALFORMED, "the peer answered about message 0x%08X where 0x%08X was asked" % [answer.id, asked.id])
		if _schema.prefix(asked.id, asked.field_count) != answer.fingerprint:
			return _refuse(FomoxaNegotiation.Verdict.CONFLICT, "message 0x%08X differs within the first %d fields" % [asked.id, asked.field_count])

	return _accept()

func _accept() -> Outcome:
	state = State.READY
	return Outcome.new() \
		.with_frame(FomoxaFrame.encode_handshake(FomoxaNegotiation.encode_verdict(FomoxaNegotiation.Verdict.ACCEPT))) \
		.with_event(FomoxaEvent.of(FomoxaEvent.Kind.READY))

func _refuse(verdict: int, why: String) -> Outcome:
	var outcome := Outcome.new().with_frame(FomoxaFrame.encode_handshake(FomoxaNegotiation.encode_verdict(verdict)))
	state = State.CLOSED
	if _terminal:
		return outcome
	_terminal = true
	return outcome.with_event(FomoxaEvent.handshake_failed(verdict, why))

func _fail(verdict: int, why: String) -> Outcome:
	state = State.CLOSED
	if _terminal:
		return Outcome.new()
	_terminal = true
	return Outcome.new().with_event(FomoxaEvent.handshake_failed(verdict, why))

func _die(reason: String) -> Outcome:
	state = State.CLOSED
	if _terminal:
		return Outcome.new()
	_terminal = true
	return Outcome.new().with_event(FomoxaEvent.disconnected(reason))
