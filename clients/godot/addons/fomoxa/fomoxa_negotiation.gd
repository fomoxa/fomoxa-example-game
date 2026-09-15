class_name FomoxaNegotiation
extends RefCounted

const PROTOCOL_VERSION := 2

const MAX_HELLO_MESSAGES := 1000000
const MAX_QUERY_ITEMS := 1000000

const HELLO_HEADER_SIZE := 16
const HELLO_ITEM_SIZE := 14
const QUERY_HEADER_SIZE := 5
const QUERY_ITEM_SIZE := 6
const REPLY_HEADER_SIZE := 4
const REPLY_ITEM_SIZE := 12

const QUERY := 4

enum Verdict {
	ACCEPT = 0,
	VERSION = 1,
	CONFLICT = 2,
	MALFORMED = 3,
}

class HelloItem:
	extends RefCounted

	var id: int
	var field_count: int
	var fingerprint: int

	func _init(message_id: int, message_field_count: int, message_fingerprint: int) -> void:
		id = message_id
		field_count = message_field_count
		fingerprint = message_fingerprint

class Hello:
	extends RefCounted

	var version: int = PROTOCOL_VERSION
	var schema_fingerprint: int = 0
	var items: Array = []

class QueryItem:
	extends RefCounted

	var id: int
	var field_count: int

	func _init(message_id: int, message_field_count: int) -> void:
		id = message_id
		field_count = message_field_count

class ReplyItem:
	extends RefCounted

	var id: int
	var fingerprint: int

	func _init(message_id: int, message_fingerprint: int) -> void:
		id = message_id
		fingerprint = message_fingerprint

static func verdict_name(verdict: int) -> String:
	match verdict:
		Verdict.ACCEPT:
			return "accepted"
		Verdict.VERSION:
			return "wrong protocol version"
		Verdict.CONFLICT:
			return "schema conflict"
		Verdict.MALFORMED:
			return "malformed hello"
		_:
			return "verdict %d" % verdict

static func encode_verdict(verdict: int) -> PackedByteArray:
	return PackedByteArray([verdict])

static func encode_hello(schema: FomoxaSchema) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(HELLO_HEADER_SIZE + HELLO_ITEM_SIZE * schema.entries.size())
	out.encode_u32(0, PROTOCOL_VERSION)
	out.encode_u64(4, schema.fingerprint)
	out.encode_u32(12, schema.entries.size())

	var at := HELLO_HEADER_SIZE
	for message in schema.entries:
		out.encode_u32(at, message.id)
		out.encode_u16(at + 4, message.field_count())
		out.encode_u64(at + 6, message.fingerprint)
		at += HELLO_ITEM_SIZE
	return out

static func decode_hello(payload: PackedByteArray) -> Array:
	if payload.size() < HELLO_HEADER_SIZE:
		return [null, "hello is %d bytes, shorter than its own header" % payload.size()]

	var count := payload.decode_u32(12)
	if count > MAX_HELLO_MESSAGES:
		return [null, "hello declares %d messages, over the 1000000 ceiling" % count]
	if payload.size() != HELLO_HEADER_SIZE + HELLO_ITEM_SIZE * count:
		return [null, "hello is %d bytes, not the %d that %d messages need" % [payload.size(), HELLO_HEADER_SIZE + HELLO_ITEM_SIZE * count, count]]

	var hello := Hello.new()
	hello.version = payload.decode_u32(0)
	hello.schema_fingerprint = payload.decode_u64(4)

	var at := HELLO_HEADER_SIZE
	for i in count:
		hello.items.append(HelloItem.new(payload.decode_u32(at), payload.decode_u16(at + 4), payload.decode_u64(at + 6)))
		at += HELLO_ITEM_SIZE
	return [hello, ""]

static func encode_query(items: Array) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(QUERY_HEADER_SIZE + QUERY_ITEM_SIZE * items.size())
	out[0] = QUERY
	out.encode_u32(1, items.size())

	var at := QUERY_HEADER_SIZE
	for item in items:
		out.encode_u32(at, item.id)
		out.encode_u16(at + 4, item.field_count)
		at += QUERY_ITEM_SIZE
	return out

static func decode_query(payload: PackedByteArray) -> Array:
	if payload.size() < QUERY_HEADER_SIZE:
		return [null, "query is %d bytes, shorter than its own header" % payload.size()]
	if payload[0] != QUERY:
		return [null, "query does not start with 4"]

	var count := payload.decode_u32(1)
	if count > MAX_QUERY_ITEMS:
		return [null, "query asks about %d messages, over the 1000000 ceiling" % count]
	if payload.size() != QUERY_HEADER_SIZE + QUERY_ITEM_SIZE * count:
		return [null, "query is %d bytes, not the %d that %d items need" % [payload.size(), QUERY_HEADER_SIZE + QUERY_ITEM_SIZE * count, count]]

	var items := []
	var at := QUERY_HEADER_SIZE
	for i in count:
		items.append(QueryItem.new(payload.decode_u32(at), payload.decode_u16(at + 4)))
		at += QUERY_ITEM_SIZE
	return [items, ""]

static func encode_reply(items: Array) -> PackedByteArray:
	var out := PackedByteArray()
	out.resize(REPLY_HEADER_SIZE + REPLY_ITEM_SIZE * items.size())
	out.encode_u32(0, items.size())

	var at := REPLY_HEADER_SIZE
	for item in items:
		out.encode_u32(at, item.id)
		out.encode_u64(at + 4, item.fingerprint)
		at += REPLY_ITEM_SIZE
	return out

static func decode_reply(payload: PackedByteArray) -> Array:
	if payload.size() < REPLY_HEADER_SIZE:
		return [null, "query reply is %d bytes, shorter than its own header" % payload.size()]

	var count := payload.decode_u32(0)
	if count > MAX_QUERY_ITEMS:
		return [null, "query reply answers %d messages, over the 1000000 ceiling" % count]
	if payload.size() != REPLY_HEADER_SIZE + REPLY_ITEM_SIZE * count:
		return [null, "query reply is %d bytes, not the %d that %d items need" % [payload.size(), REPLY_HEADER_SIZE + REPLY_ITEM_SIZE * count, count]]

	var items := []
	var at := REPLY_HEADER_SIZE
	for i in count:
		items.append(ReplyItem.new(payload.decode_u32(at), payload.decode_u64(at + 4)))
		at += REPLY_ITEM_SIZE
	return [items, ""]
