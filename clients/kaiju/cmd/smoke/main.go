package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	"kaijuengine.com/fomoxa_example/network"
	"kaijuengine.com/fomoxa_example/network/models"
)

const (
	frame       = 16 * time.Millisecond
	jumpHeight  = float32(0.5)
	lookYaw     = float32(0.75)
	lookPitch   = float32(-0.4)
	lookEpsilon = float32(0.001)
)

func main() {
	address := flag.String("addr", "127.0.0.1:9321", "server address")
	name := flag.String("name", "kaiju-smoke", "display name")
	seconds := flag.Float64("seconds", 4, "how long to run")
	expectPlayers := flag.Int("expect-players", 1, "players expected in a snapshot")
	flag.Parse()

	session := network.NewSession()
	if err := session.Open(*address, *name); err != nil {
		fail(err.Error())
	}
	defer session.Close()

	started := time.Now()
	deadline := started.Add(time.Duration(*seconds * float64(time.Second)))
	var (
		sawItself    bool
		mostPlayers  int
		startX       float32
		movedEast    bool
		jumped       bool
		lookEchoed   bool
		haveStartX   bool
		snapshotSeen bool
	)

	for now := time.Now(); now.Before(deadline); now = time.Now() {
		if err := session.Poll(now); err != nil {
			fail(err.Error())
		}

		if session.Joined() {
			if err := session.SendInput(1, 0, true, lookYaw, lookPitch); err != nil {
				fail(err.Error())
			}
		}

		if count := session.PlayerCount(); count > mostPlayers {
			mostPlayers = count
		}
		if state, ok := session.LocalPlayer(); ok {
			snapshotSeen = true
			sawItself = true
			if !haveStartX {
				startX = state.PositionX
				haveStartX = true
			}
			movedEast = movedEast || state.PositionX > startX+0.05
			jumped = jumped || state.PositionY > jumpHeight
			lookEchoed = lookEchoed || matches(state, lookYaw, lookPitch)
		}

		time.Sleep(frame)
	}

	passed := snapshotSeen && sawItself && movedEast && jumped && lookEchoed && mostPlayers >= *expectPlayers
	fmt.Printf(
		"%s: %s (players %d of %d expected, saw itself %t, moved +X %t, jumped %t, look echoed %t)\n",
		*name,
		verdict(passed),
		mostPlayers,
		*expectPlayers,
		sawItself,
		movedEast,
		jumped,
		lookEchoed,
	)
	if !passed {
		os.Exit(1)
	}
}

func matches(state models.PlayerState, yaw, pitch float32) bool {
	return absDelta(state.LookYaw, network.WrapAngle(yaw)) < lookEpsilon &&
		absDelta(state.LookPitch, network.ClampPitch(pitch)) < lookEpsilon
}

func absDelta(a, b float32) float32 {
	if a > b {
		return a - b
	}
	return b - a
}

func verdict(passed bool) string {
	if passed {
		return "PASS"
	}
	return "FAIL"
}

func fail(reason string) {
	fmt.Fprintf(os.Stderr, "kaiju smoke: %s\n", reason)
	os.Exit(1)
}
