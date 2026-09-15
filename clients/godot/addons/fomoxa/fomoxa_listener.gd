class_name FomoxaListener
extends RefCounted

func accept(_budget: int) -> Array:
	push_error("fomoxa: a listener must implement accept()")
	return []

func local_port() -> int:
	return 0

func close() -> void:
	pass

func failure() -> String:
	return ""
