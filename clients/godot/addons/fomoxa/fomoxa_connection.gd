class_name FomoxaConnection
extends RefCounted

enum Result {
	OK,
	NOT_READY,
	CONGESTED,
	PAYLOAD_TOO_LARGE,
	TRANSPORT_LIMIT,
	CLOSED,
}

static func result_name(result: int) -> String:
	match result:
		Result.OK:
			return "ok"
		Result.NOT_READY:
			return "the handshake has not finished"
		Result.CONGESTED:
			return "a frame is still on its way out"
		Result.PAYLOAD_TOO_LARGE:
			return "the payload is over Fomoxa's 16 MiB ceiling"
		Result.TRANSPORT_LIMIT:
			return "the frame is over this transport's own ceiling"
		Result.CLOSED:
			return "the session is closed"
		_:
			return "result %d" % result

var peer_id: int = 0

var _transport: FomoxaTransport
var _session: FomoxaSession
var _source: FomoxaFrameSource
var _config: FomoxaConfig

var _events: Array = []
## One data frame plus the handful of control frames the protocol can owe at
## any moment.
const MAX_PENDING_BYTES := 64 * 1024

var _pending: Array = []
var _pending_bytes_total := 0
var _started := false
var _dead := false
var _dead_reason := ""
var _released := false

static func client(transport: FomoxaTransport, schema: FomoxaSchema, config: FomoxaConfig = null) -> FomoxaConnection:
	return FomoxaConnection.new(transport, schema, config, FomoxaSession.Role.CLIENT, 0)

static func peer(transport: FomoxaTransport, schema: FomoxaSchema, config: FomoxaConfig = null, id: int = 0) -> FomoxaConnection:
	return FomoxaConnection.new(transport, schema, config, FomoxaSession.Role.SERVER, id)

func _init(transport: FomoxaTransport, schema: FomoxaSchema, config: FomoxaConfig, role: int, id: int) -> void:
	_config = FomoxaConfig.new() if config == null else config.normalized()
	_transport = transport
	_session = FomoxaSession.new(role, schema, _config)
	_source = FomoxaFrameSource.new(transport, _config.read_capacity)
	peer_id = id

func state() -> int:
	return _session.state

func ready() -> bool:
	return _session.state == FomoxaSession.State.READY

func finished() -> bool:
	return _session.closed() and _session.terminal_reported()

func send(message_id: int, payload: PackedByteArray) -> int:
	if _session.state != FomoxaSession.State.READY:
		return Result.CLOSED if _session.closed() else Result.NOT_READY
	if payload.size() > FomoxaFrame.MAX_MESSAGE_PAYLOAD:
		return Result.PAYLOAD_TOO_LARGE

	var frame := FomoxaFrame.encode_data(message_id, payload)
	if not _pending.is_empty():
		return Result.CONGESTED

	match _transport.send(frame):
		FomoxaTransport.Status.OK:
			return Result.OK
		FomoxaTransport.Status.PENDING:
			_pending.append(frame)
			return Result.OK
		FomoxaTransport.Status.TOO_LARGE:
			return Result.TRANSPORT_LIMIT
		_:
			_mark_dead("the transport went away while sending")
			return Result.CLOSED

func close() -> void:
	if _session.closed():
		return
	_session.close()
	_transport.close_send()

func release() -> void:
	if _released:
		return
	_released = true
	_transport.close()

func poll(now_msec: int) -> Array:
	_events = []
	if _released:
		return _events

	if not _started:
		_started = true
		_events.append(FomoxaEvent.of(FomoxaEvent.Kind.CONNECTED))
		_apply(_session.start(now_msec))

	_flush()

	if not _dead and not _session.closed():
		_drain(now_msec)

	if not _dead and not _session.closed():
		_apply(_session.tick(now_msec))

	if _dead and not _session.closed():
		_apply(_session.transport_closed(_dead_reason))

	return _events

func _flush() -> void:
	while not _pending.is_empty() and not _dead:
		match _transport.send(_pending[0]):
			FomoxaTransport.Status.OK:
				_pending_bytes_total -= _pending[0].size()
				_pending.pop_front()
			FomoxaTransport.Status.PENDING:
				return
			FomoxaTransport.Status.TOO_LARGE:
				_pending_bytes_total -= _pending[0].size()
				_pending.pop_front()
				_mark_dead("the transport cannot carry a frame this protocol requires")
			_:
				_mark_dead("the transport went away while sending")

func _drain(now_msec: int) -> void:
	var budget := _config.max_frames_per_poll
	while budget > 0:
		var taken := _source.next()
		match taken[0]:
			FomoxaFrameSource.Result.FRAME:
				budget -= 1
				_apply(_session.handle_frame(taken[1], now_msec))
				if _dead or _session.closed():
					return
			FomoxaFrameSource.Result.DROPPED:
				budget -= 1
			FomoxaFrameSource.Result.EMPTY:
				return
			FomoxaFrameSource.Result.CLOSED:
				_mark_dead("the peer closed the connection")
				return
			_:
				_mark_dead(_source.failure())
				return

func _apply(outcome: FomoxaSession.Outcome) -> void:
	if not outcome.frame.is_empty():
		_write(outcome.frame)

	if outcome.event != null:
		outcome.event.peer = peer_id
		_events.append(outcome.event)
		if outcome.event.kind == FomoxaEvent.Kind.HANDSHAKE_FAILED:
			_dead = true
			_transport.close_send()

func _write(frame: PackedByteArray) -> void:
	if _dead:
		return
	# Queue behind whatever is already waiting, never overwrite it: a refusal
	# verdict lost that way leaves the peer waiting out its deadline without
	# ever learning why. The protocol caps how many control frames can be owed
	# at once - one probe per silence window, one reply per probe, at most one
	# query round per session - so passing the ceiling means an assumption
	# broke. The peer stopped reading, which is the same "not keeping up" a
	# heartbeat timeout reports; the transport itself is fine. (02 §8)
	if _pending_bytes_total + frame.size() > MAX_PENDING_BYTES:
		_mark_dead("the peer stopped reading and the pending queue reached its ceiling")
		return
	_pending.append(frame)
	_pending_bytes_total += frame.size()
	_flush()

func _mark_dead(reason: String) -> void:
	if _dead:
		return
	_dead = true
	_dead_reason = reason if reason != "" else "the transport went away"
