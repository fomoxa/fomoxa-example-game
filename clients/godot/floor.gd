@tool
extends Node3D

const GRID_SPACING := 2.0
const GRID_LINE_WIDTH := 0.05
const GRID_LINE_HEIGHT := 0.01
const FLOOR_COLOR := Color(0.22, 0.24, 0.28)
const GRID_COLOR := Color(0.38, 0.41, 0.47)

@export var half_size := 10.0:
	set(value):
		half_size = value
		_rebuild()

func _ready() -> void:
	_rebuild()

func _rebuild() -> void:
	if not is_inside_tree():
		return
	for child in get_children():
		remove_child(child)
		child.queue_free()

	var extent := half_size * 2.0
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2.ONE * extent
	ground.mesh = plane
	ground.material_override = _flat_material(FLOOR_COLOR)
	add_child(ground)

	var line_material := _flat_material(GRID_COLOR)
	var line_count: int = int(extent / GRID_SPACING) + 1
	for index: int in line_count:
		var offset: float = -half_size + index * GRID_SPACING
		add_child(_box(Vector3(GRID_LINE_WIDTH, GRID_LINE_HEIGHT, extent), Vector3(offset, GRID_LINE_HEIGHT * 0.5, 0.0), line_material))
		add_child(_box(Vector3(extent, GRID_LINE_HEIGHT, GRID_LINE_WIDTH), Vector3(0.0, GRID_LINE_HEIGHT * 0.5, offset), line_material))

func _box(size: Vector3, offset: Vector3, material: Material) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	var mesh := BoxMesh.new()
	mesh.size = size
	instance.mesh = mesh
	instance.position = offset
	instance.material_override = material
	return instance

func _flat_material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	return material
