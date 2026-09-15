class_name FomoxaSchema
extends RefCounted

const MAX_FIELD_COUNT := 65535

enum Check {
	MATCH,
	REJECT,
	NEED_PREFIX,
}

class Entry:
	extends RefCounted

	var id: int
	var fingerprint: int
	var prefixes: Array

	func _init(message_id: int, message_fingerprint: int, message_prefixes: Array) -> void:
		id = message_id
		fingerprint = message_fingerprint
		prefixes = message_prefixes

	func field_count() -> int:
		return prefixes.size()

var fingerprint: int = 0
var entries: Array = []

var _by_id := {}

static func entry(id: int, message_fingerprint: int, prefixes: Array) -> Entry:
	return Entry.new(id, message_fingerprint, prefixes)

static func create(schema_fingerprint: int, schema_entries: Array) -> Array:
	var schema := FomoxaSchema.new()
	schema.fingerprint = schema_fingerprint

	for item in schema_entries:
		if item.prefixes.size() > MAX_FIELD_COUNT:
			return [null, "message 0x%08X declares %d fields, over the 65535 the wire can carry" % [item.id, item.prefixes.size()]]
		if not item.prefixes.is_empty() and item.prefixes[item.prefixes.size() - 1] != item.fingerprint:
			return [null, "message 0x%08X: the last prefix is not the message fingerprint" % item.id]
		if schema._by_id.has(item.id):
			return [null, "message id 0x%08X is declared twice" % item.id]
		schema._by_id[item.id] = item
		schema.entries.append(item)

	schema.entries.sort_custom(func(a, b): return a.id < b.id)
	return [schema, ""]

static func from_generated(schema_fingerprint: int, messages: Array) -> Array:
	var converted := []
	for message in messages:
		converted.append(Entry.new(message.id, message.fingerprint, message.prefixes))
	return create(schema_fingerprint, converted)

func message(id: int) -> Entry:
	var known: Entry = _by_id.get(id)
	return known

func declares(id: int) -> bool:
	return _by_id.has(id)

func prefix(id: int, field_count: int) -> int:
	var known: Entry = _by_id.get(id)
	if known == null or field_count <= 0 or field_count > known.prefixes.size():
		return 0
	return known.prefixes[field_count - 1]

func has_prefix(id: int, field_count: int) -> bool:
	var known: Entry = _by_id.get(id)
	return known != null and field_count >= 1 and field_count <= known.prefixes.size()

func check_message(id: int, peer_field_count: int, peer_fingerprint: int) -> Array:
	var known: Entry = _by_id.get(id)
	if known == null:
		return [Check.MATCH, 0]

	var local_field_count := known.field_count()
	if peer_fingerprint == known.fingerprint:
		return [Check.MATCH, 0]
	if peer_field_count == 0 or local_field_count == 0:
		return [Check.MATCH, 0]
	if peer_field_count == local_field_count:
		return [Check.REJECT, 0]
	if peer_field_count < local_field_count:
		if known.prefixes[peer_field_count - 1] == peer_fingerprint:
			return [Check.MATCH, 0]
		return [Check.REJECT, 0]
	return [Check.NEED_PREFIX, local_field_count]
