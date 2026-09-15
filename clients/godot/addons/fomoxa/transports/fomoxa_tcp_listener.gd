class_name FomoxaTcpListener
extends FomoxaListener

var _server: TCPServer

static func bind_to(port: int, address: String = "*") -> Array:
	var server := TCPServer.new()
	var error := server.listen(port, address)
	if error != OK:
		return [null, error]
	var listener := FomoxaTcpListener.new()
	listener._server = server
	return [listener, OK]

func accept(budget: int) -> Array:
	var arrivals: Array = []
	var room := budget
	while room > 0 and _server.is_connection_available():
		var peer := _server.take_connection()
		if peer == null:
			break
		arrivals.append(FomoxaTcpTransport.over(peer))
		room -= 1
	return arrivals

func local_port() -> int:
	return _server.get_local_port()

func close() -> void:
	_server.stop()
