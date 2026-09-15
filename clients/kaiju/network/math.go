package network

import "math"

const (
	MaxLookPitch = float32(1.5)
	halfTurn     = float32(math.Pi)
)

func WorldMove(strafe, forward, lookYaw float32) (float32, float32) {
	sin := float32(math.Sin(float64(lookYaw)))
	cos := float32(math.Cos(float64(lookYaw)))
	return cos*strafe - sin*forward, -sin*strafe - cos*forward
}

func WrapAngle(angle float32) float32 {
	wrapped := float32(math.Mod(float64(angle+halfTurn), float64(2*halfTurn)))
	if wrapped < 0 {
		wrapped += 2 * halfTurn
	}
	return wrapped - halfTurn
}

func ClampPitch(pitch float32) float32 {
	if pitch < -MaxLookPitch {
		return -MaxLookPitch
	}
	if pitch > MaxLookPitch {
		return MaxLookPitch
	}
	return pitch
}

func LookDirection(lookYaw, lookPitch float32) (float32, float32, float32) {
	cosPitch := float32(math.Cos(float64(lookPitch)))
	return -float32(math.Sin(float64(lookYaw))) * cosPitch,
		float32(math.Sin(float64(lookPitch))),
		-float32(math.Cos(float64(lookYaw))) * cosPitch
}
