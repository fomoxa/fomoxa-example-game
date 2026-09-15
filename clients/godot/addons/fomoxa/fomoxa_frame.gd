class_name FomoxaFrame
extends RefCounted

enum Type {
	DATA = 0,
	PROBE = 1,
	ACK = 2,
	HANDSHAKE = 3,
}

const MAGIC_C := 0x43
const MAGIC_Y := 0x59

const DATA_HEADER_SIZE := 11
const HANDSHAKE_HEADER_SIZE := 5

const MAX_MESSAGE_PAYLOAD := 16 * 1024 * 1024
const MAX_HANDSHAKE_PAYLOAD := 1024 * 1024
const MAX_FRAME_SIZE := DATA_HEADER_SIZE + MAX_MESSAGE_PAYLOAD

const INCOMPLETE := "incomplete"

var type: int = Type.DATA
var message_id: int = 0
var payload := PackedByteArray()

static func data(id: int, body: PackedByteArray) -> FomoxaFrame:
	var frame := FomoxaFrame.new()
	frame.type = Type.DATA
	frame.message_id = id
	frame.payload = body
	return frame

static func handshake(body: PackedByteArray) -> FomoxaFrame:
	var frame := FomoxaFrame.new()
	frame.type = Type.HANDSHAKE
	frame.payload = body
	return frame

static func control(frame_type: int) -> FomoxaFrame:
	var frame := FomoxaFrame.new()
	frame.type = frame_type
	return frame

static func encode_data(id: int, body: PackedByteArray) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(DATA_HEADER_SIZE)
	out[0] = Type.DATA
	out[1] = MAGIC_C
	out[2] = MAGIC_Y
	out.encode_u32(3, id)
	out.encode_u32(7, body.size())
	out.append_array(body)
	return out

static func encode_handshake(body: PackedByteArray) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(HANDSHAKE_HEADER_SIZE)
	out[0] = Type.HANDSHAKE
	out.encode_u32(1, body.size())
	out.append_array(body)
	return out

static func encode_probe() -> PackedByteArray:
	return PackedByteArray([Type.PROBE])

static func encode_ack() -> PackedByteArray:
	return PackedByteArray([Type.ACK])

static func encode(frame: FomoxaFrame) -> PackedByteArray:
	match frame.type:
		Type.DATA:
			return encode_data(frame.message_id, frame.payload)
		Type.HANDSHAKE:
			return encode_handshake(frame.payload)
		Type.PROBE:
			return encode_probe()
		_:
			return encode_ack()

static func decode(bytes: PackedByteArray, offset: int) -> Array:
	var available := bytes.size() - offset
	if available < 1:
		return [null, 0, INCOMPLETE]

	var frame_type := bytes[offset]
	match frame_type:
		Type.PROBE, Type.ACK:
			return [control(frame_type), 1, ""]

		Type.DATA:
			if available < DATA_HEADER_SIZE:
				return [null, 0, INCOMPLETE]
			if bytes[offset + 1] != MAGIC_C or bytes[offset + 2] != MAGIC_Y:
				return [null, 0, "data frame without the CY marker"]
			var length := bytes.decode_u32(offset + 7)
			if length > MAX_MESSAGE_PAYLOAD:
				return [null, 0, "message payload of %d bytes is over the 16 MiB ceiling" % length]
			var total := DATA_HEADER_SIZE + length
			if available < total:
				return [null, 0, INCOMPLETE]
			var frame := data(bytes.decode_u32(offset + 3), bytes.slice(offset + DATA_HEADER_SIZE, offset + total))
			return [frame, total, ""]

		Type.HANDSHAKE:
			if available < HANDSHAKE_HEADER_SIZE:
				return [null, 0, INCOMPLETE]
			var length := bytes.decode_u32(offset + 1)
			if length > MAX_HANDSHAKE_PAYLOAD:
				return [null, 0, "handshake payload of %d bytes is over the 1 MiB ceiling" % length]
			var total := HANDSHAKE_HEADER_SIZE + length
			if available < total:
				return [null, 0, INCOMPLETE]
			var frame := handshake(bytes.slice(offset + HANDSHAKE_HEADER_SIZE, offset + total))
			return [frame, total, ""]

		_:
			return [null, 0, "frame type %d is not one of 0-3" % frame_type]

static func decode_packet(bytes: PackedByteArray) -> Array:
	var result := decode(bytes, 0)
	if result[2] == INCOMPLETE:
		return [null, "packet ends before the frame it declares"]
	if result[2] != "":
		return [null, result[2]]
	if result[1] != bytes.size():
		return [null, "packet carries %d bytes after its frame" % (bytes.size() - result[1])]
	return [result[0], ""]

class Decoder:
	extends RefCounted

	var _buffer := PackedByteArray()
	var _offset := 0
	var _poison := ""

	func feed(bytes: PackedByteArray) -> void:
		if _offset > 0 and _offset == _buffer.size():
			_buffer = bytes
			_offset = 0
			return
		if _offset > 0:
			_buffer = _buffer.slice(_offset)
			_offset = 0
		_buffer.append_array(bytes)

	func next() -> Array:
		if _poison != "":
			return [null, _poison]
		var result := FomoxaFrame.decode(_buffer, _offset)
		if result[2] == FomoxaFrame.INCOMPLETE:
			return [null, ""]
		if result[2] != "":
			_poison = result[2]
			return [null, _poison]
		_offset += result[1]
		return [result[0], ""]

	func poisoned() -> bool:
		return _poison != ""

	func buffered() -> int:
		return _buffer.size() - _offset
