package network

import (
	"fmt"

	fomoxa "github.com/fomoxa/go"
	"kaijuengine.com/fomoxa_example/network/generated"
	"kaijuengine.com/fomoxa_example/network/models"
)

const (
	ClientKindBot    = uint8(0)
	ClientKindUnity  = uint8(1)
	ClientKindGodot  = uint8(2)
	ClientKindUnreal = uint8(3)
	ClientKindKaiju  = uint8(4)
)

func Schema() (*fomoxa.Schema, error) {
	messages := make([]fomoxa.Message, 0, len(generated.FomoxaMessages))
	for _, message := range generated.FomoxaMessages {
		messages = append(messages, fomoxa.Message{
			ID:          message.ID,
			Fingerprint: message.Fingerprint,
			Prefixes:    message.Prefixes,
		})
	}
	return fomoxa.NewSchema(generated.FomoxaSchemaFingerprint, messages)
}

func EncodeHello(hello *models.ClientHello) []byte {
	writer := generated.NewWriter()
	generated.ClientHelloGameCodec{}.Encode(writer, hello)
	return writer.Bytes()
}

func EncodeInput(input *models.PlayerInput) []byte {
	writer := generated.NewWriter()
	generated.PlayerInputGameCodec{}.Encode(writer, input)
	return writer.Bytes()
}

func DecodeWelcome(payload []byte) (models.Welcome, error) {
	var welcome models.Welcome
	err := generated.WelcomeGameCodec{}.Decode(generated.NewReader(payload), &welcome)
	return welcome, err
}

func DecodeSnapshot(payload []byte) (models.WorldSnapshot, error) {
	var snapshot models.WorldSnapshot
	err := generated.WorldSnapshotGameCodec{}.Decode(generated.NewReader(payload), &snapshot)
	return snapshot, err
}

func KindName(kind uint8) string {
	switch kind {
	case ClientKindBot:
		return "bot"
	case ClientKindUnity:
		return "unity"
	case ClientKindGodot:
		return "godot"
	case ClientKindUnreal:
		return "unreal"
	case ClientKindKaiju:
		return "kaiju"
	default:
		return "unknown"
	}
}

func PlayerLabel(state models.PlayerState) string {
	return fmt.Sprintf("#%d %s", state.PlayerID, KindName(state.ClientKind))
}
