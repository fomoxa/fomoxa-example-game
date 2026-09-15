class_name FomoxaConfig
extends RefCounted

var handshake_timeout_msec: int = 5000
var heartbeat_interval_msec: int = 5000
var heartbeat_timeout_msec: int = 15000
var max_frames_per_poll: int = 64
var read_capacity: int = 64 * 1024
var max_accepts_per_poll: int = 16

func copy() -> FomoxaConfig:
	var other := FomoxaConfig.new()
	other.handshake_timeout_msec = handshake_timeout_msec
	other.heartbeat_interval_msec = heartbeat_interval_msec
	other.heartbeat_timeout_msec = heartbeat_timeout_msec
	other.max_frames_per_poll = max_frames_per_poll
	other.read_capacity = read_capacity
	other.max_accepts_per_poll = max_accepts_per_poll
	return other

func normalized() -> FomoxaConfig:
	var other := copy()
	if other.handshake_timeout_msec <= 0:
		other.handshake_timeout_msec = 5000
	if other.heartbeat_interval_msec <= 0:
		other.heartbeat_interval_msec = 5000
	if other.heartbeat_timeout_msec <= 0:
		other.heartbeat_timeout_msec = 15000
	if other.max_frames_per_poll <= 0:
		other.max_frames_per_poll = 64
	if other.read_capacity <= 0:
		other.read_capacity = 64 * 1024
	if other.max_accepts_per_poll <= 0:
		other.max_accepts_per_poll = 16
	return other
