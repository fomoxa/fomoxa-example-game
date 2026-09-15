class_name FomoxaTcp
extends RefCounted

static func connect_to_host(host: String, port: int, schema: FomoxaSchema, config: FomoxaConfig = null) -> Array:
	var made := FomoxaTcpTransport.connect_to(host, port)
	if made[1] != OK:
		return [null, made[1]]
	return [FomoxaConnection.client(made[0], schema, config), OK]

static func listen(port: int, schema: FomoxaSchema, config: FomoxaConfig = null, address: String = "*") -> Array:
	var made := FomoxaTcpListener.bind_to(port, address)
	if made[1] != OK:
		return [null, made[1]]
	return [FomoxaServer.over(made[0], schema, config), OK]
