extends SceneTree

const SENT_LOOK_YAW := 0.5
const SENT_LOOK_PITCH := 0.25
const MINIMUM_JUMP_HEIGHT := 0.5

func _init() -> void:
	var options := FomoxaExampleProtocol.read_options({
		"host": FomoxaExampleProtocol.DEFAULT_HOST,
		"port": FomoxaExampleProtocol.DEFAULT_PORT,
		"name": "godot-smoke",
		"seconds": 3.0,
		"expect-players": 1,
	})
	var session := FomoxaExampleSession.new()
	if session.open(options.host, options.port, options.name) != OK:
		_finish(session, false, session.failure_reason)
		return

	var deadline := Time.get_ticks_msec() + int(options.seconds * 1000.0)
	var start_x := NAN
	var last_x := NAN
	var highest_y := 0.0
	var last_yaw := NAN
	var last_pitch := NAN
	var most_players := 0
	while Time.get_ticks_msec() < deadline:
		var now := Time.get_ticks_msec()
		session.poll(now)
		if session.status == FomoxaExampleSession.Status.FAILED:
			break
		session.update_input(now, Vector2(1.0, 0.0), true, SENT_LOOK_YAW, SENT_LOOK_PITCH)
		var own := session.local_state()
		if own != null:
			if is_nan(start_x):
				start_x = own.position_x
			last_x = own.position_x
			highest_y = maxf(highest_y, own.position_y)
			last_yaw = own.look_yaw
			last_pitch = own.look_pitch
		most_players = maxi(most_players, session.players.size())
		OS.delay_msec(5)

	var failures: PackedStringArray = []
	if session.status != FomoxaExampleSession.Status.JOINED:
		failures.append("not joined (%s)" % session.failure_reason)
	if session.snapshots_received == 0:
		failures.append("no WorldSnapshot received")
	if most_players < options["expect-players"]:
		failures.append("saw at most %d players, expected %d" % [most_players, options["expect-players"]])
	if is_nan(start_x) or last_x - start_x < 1.0:
		failures.append("own player did not move along +x (start %s, last %s)" % [start_x, last_x])
	if highest_y < MINIMUM_JUMP_HEIGHT:
		failures.append("own player never jumped above %.2f (highest %.2f)" % [MINIMUM_JUMP_HEIGHT, highest_y])
	if is_nan(last_yaw) or not is_equal_approx(last_yaw, SENT_LOOK_YAW) or not is_equal_approx(last_pitch, SENT_LOOK_PITCH):
		failures.append("look direction not echoed (yaw %s, pitch %s)" % [last_yaw, last_pitch])

	var report := "player #%d · %d snapshots · most players %d · x %.2f -> %.2f · highest y %.2f · look (%.2f, %.2f)" % [session.local_player_id, session.snapshots_received, most_players, start_x, last_x, highest_y, last_yaw, last_pitch]
	_finish(session, failures.is_empty(), report if failures.is_empty() else "; ".join(failures))

func _finish(session: FomoxaExampleSession, passed: bool, detail: String) -> void:
	session.close()
	print("godot smoke test: %s - %s" % ["PASS" if passed else "FAIL", detail])
	quit(0 if passed else 1)
