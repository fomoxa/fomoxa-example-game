package network

import (
	"errors"
	"fmt"
	"time"

	fomoxa "github.com/fomoxa/go"
	"kaijuengine.com/fomoxa_example/network/generated"
	"kaijuengine.com/fomoxa_example/network/models"
)

type Status int

const (
	StatusIdle Status = iota
	StatusConnecting
	StatusJoined
	StatusFailed
)

func (s Status) String() string {
	switch s {
	case StatusConnecting:
		return "connecting"
	case StatusJoined:
		return "joined"
	case StatusFailed:
		return "failed"
	default:
		return "idle"
	}
}

type Session struct {
	conn          *fomoxa.Conn
	status        Status
	failure       string
	displayName   string
	helloPending  bool
	sequence      uint32
	localID       uint32
	planeHalfSize float32
	tickRate      uint16
	tick          uint32
	players       map[uint32]models.PlayerState
	order         []uint32
}

func NewSession() *Session {
	return &Session{players: make(map[uint32]models.PlayerState)}
}

func (s *Session) Status() string         { return s.status.String() }
func (s *Session) Failure() string        { return s.failure }
func (s *Session) Joined() bool           { return s.status == StatusJoined }
func (s *Session) LocalID() uint32        { return s.localID }
func (s *Session) PlaneHalfSize() float32 { return s.planeHalfSize }
func (s *Session) TickRate() uint16       { return s.tickRate }
func (s *Session) Tick() uint32           { return s.tick }
func (s *Session) PlayerCount() int       { return len(s.players) }
func (s *Session) PlayerIDs() []uint32    { return s.order }

func (s *Session) Player(id uint32) (models.PlayerState, bool) {
	state, ok := s.players[id]
	return state, ok
}

func (s *Session) LocalPlayer() (models.PlayerState, bool) {
	return s.Player(s.localID)
}

func (s *Session) Open(address, displayName string) error {
	s.Close()
	schema, err := Schema()
	if err != nil {
		return s.fail(fmt.Sprintf("cannot build the schema: %v", err))
	}
	conn, err := fomoxa.DialTCP(address, schema, fomoxa.DefaultConfig())
	if err != nil {
		return s.fail(fmt.Sprintf("cannot reach %s: %v", address, err))
	}
	s.conn = conn
	s.displayName = displayName
	s.status = StatusConnecting
	s.failure = ""
	s.helloPending = false
	s.sequence = 0
	s.localID = 0
	s.tick = 0
	s.players = make(map[uint32]models.PlayerState)
	s.order = nil
	return nil
}

func (s *Session) Close() {
	if s.conn != nil {
		_ = s.conn.Close()
		s.conn = nil
	}
	s.status = StatusIdle
	s.helloPending = false
}

func (s *Session) Poll(now time.Time) error {
	if s.conn == nil {
		return nil
	}

	for _, event := range s.conn.Tick(now) {
		switch event.Kind {
		case fomoxa.EventReady:
			s.helloPending = true
		case fomoxa.EventMessage:
			if err := s.receive(event); err != nil {
				return err
			}
		case fomoxa.EventHandshakeFailed:
			return s.fail(fmt.Sprintf("handshake refused (%s): %v", event.Verdict, event.Err))
		case fomoxa.EventDisconnected:
			return s.fail(fmt.Sprintf("disconnected: %v", event.Err))
		}
	}

	if s.helloPending {
		hello := models.ClientHello{ClientKind: ClientKindKaiju, DisplayName: s.displayName}
		switch err := s.conn.Send(generated.ClientHelloGameCodecMessageID, EncodeHello(&hello)); {
		case err == nil:
			s.helloPending = false
		case errors.Is(err, fomoxa.ErrCongested):
		default:
			return s.fail(fmt.Sprintf("cannot send ClientHello: %v", err))
		}
	}
	return nil
}

func (s *Session) SendInput(moveX, moveZ float32, jump bool, lookYaw, lookPitch float32) error {
	if s.conn == nil || s.status != StatusJoined {
		return nil
	}
	s.sequence++
	input := models.PlayerInput{
		Sequence:  s.sequence,
		MoveX:     moveX,
		MoveZ:     moveZ,
		Jump:      jump,
		LookYaw:   WrapAngle(lookYaw),
		LookPitch: ClampPitch(lookPitch),
	}
	switch err := s.conn.Send(generated.PlayerInputGameCodecMessageID, EncodeInput(&input)); {
	case err == nil, errors.Is(err, fomoxa.ErrCongested), errors.Is(err, fomoxa.ErrNotReady):
		return nil
	default:
		return s.fail(fmt.Sprintf("cannot send PlayerInput: %v", err))
	}
}

func (s *Session) receive(event fomoxa.Event) error {
	switch event.MessageID {
	case generated.WelcomeGameCodecMessageID:
		welcome, err := DecodeWelcome(event.Payload)
		if err != nil {
			return s.fail(fmt.Sprintf("undecodable Welcome: %v", err))
		}
		s.localID = welcome.PlayerID
		s.planeHalfSize = welcome.PlaneHalfSize
		s.tickRate = welcome.TickRate
		s.status = StatusJoined
	case generated.WorldSnapshotGameCodecMessageID:
		snapshot, err := DecodeSnapshot(event.Payload)
		if err != nil {
			return s.fail(fmt.Sprintf("undecodable WorldSnapshot: %v", err))
		}
		s.tick = snapshot.Tick
		s.order = s.order[:0]
		seen := make(map[uint32]struct{}, len(snapshot.Players))
		for _, state := range snapshot.Players {
			s.players[state.PlayerID] = state
			s.order = append(s.order, state.PlayerID)
			seen[state.PlayerID] = struct{}{}
		}
		for id := range s.players {
			if _, ok := seen[id]; !ok {
				delete(s.players, id)
			}
		}
	}
	return nil
}

func (s *Session) fail(reason string) error {
	s.status = StatusFailed
	s.failure = reason
	if s.conn != nil {
		_ = s.conn.Close()
		s.conn = nil
	}
	return errors.New(reason)
}
