package models

//fomoxa:model codec=game
type ClientHello struct {
	ClientKind  uint8  `fomoxa:"u8" codec:"game"`
	DisplayName string `fomoxa:"string" codec:"game"`
}

//fomoxa:model codec=game
type Welcome struct {
	PlayerID      uint32  `fomoxa:"u32" codec:"game"`
	PlaneHalfSize float32 `fomoxa:"f32" codec:"game"`
	TickRate      uint16  `fomoxa:"u16" codec:"game"`
}

//fomoxa:model codec=game
type PlayerInput struct {
	Sequence  uint32  `fomoxa:"u32" codec:"game"`
	MoveX     float32 `fomoxa:"f32" codec:"game"`
	MoveZ     float32 `fomoxa:"f32" codec:"game"`
	Jump      bool    `fomoxa:"bool" codec:"game"`
	LookYaw   float32 `fomoxa:"f32" codec:"game"`
	LookPitch float32 `fomoxa:"f32" codec:"game"`
}

//fomoxa:model codec=game
type PlayerState struct {
	PlayerID   uint32  `fomoxa:"u32" codec:"game"`
	ClientKind uint8   `fomoxa:"u8" codec:"game"`
	Color      uint32  `fomoxa:"u32" codec:"game"`
	PositionX  float32 `fomoxa:"f32" codec:"game"`
	PositionZ  float32 `fomoxa:"f32" codec:"game"`
	PositionY  float32 `fomoxa:"f32" codec:"game"`
	LookYaw    float32 `fomoxa:"f32" codec:"game"`
	LookPitch  float32 `fomoxa:"f32" codec:"game"`
}

//fomoxa:model codec=game
type WorldSnapshot struct {
	Tick    uint32        `fomoxa:"u32" codec:"game"`
	Players []PlayerState `fomoxa:"Array<PlayerState>" codec:"game"`
}
