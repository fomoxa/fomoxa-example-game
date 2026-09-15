using System;
using FomoxaExample.Networking.Models;
using Fomoxa.Net;
using Generated;

namespace FomoxaExample.Networking
{
    public static class FomoxaExampleProtocol
    {
        public const string DefaultHost = "127.0.0.1";
        public const int DefaultPort = 9321;
        public const byte ClientKindUnity = 1;
        public const float MaxLookPitch = 1.5f;

        static readonly string[] ClientKindNames = { "Bot", "Unity", "Godot", "Unreal", "Kaiju", "nunuStudio" };

        public static Schema BuildSchema()
        {
            var messages = new MessageSchema[Handshake.FomoxaMessages.Length];
            for (var index = 0; index < messages.Length; index++)
            {
                var message = Handshake.FomoxaMessages[index];
                messages[index] = new MessageSchema(message.Id, message.Fingerprint, message.Prefixes);
            }
            return new Schema(Handshake.FomoxaSchemaFingerprint, messages);
        }

        public static byte[] EncodeHello(string displayName)
        {
            var writer = new Writer();
            ClientHelloGameCodec.Encode(writer, new ClientHello { ClientKind = ClientKindUnity, DisplayName = displayName });
            return writer.ToArray();
        }

        public static byte[] EncodeInput(PlayerInput input)
        {
            var writer = new Writer();
            PlayerInputGameCodec.Encode(writer, input);
            return writer.ToArray();
        }

        public static Welcome DecodeWelcome(ReadOnlySpan<byte> payload)
        {
            var reader = new Reader(payload);
            var welcome = new Welcome();
            WelcomeGameCodec.Decode(ref reader, ref welcome);
            return welcome;
        }

        public static WorldSnapshot DecodeSnapshot(ReadOnlySpan<byte> payload)
        {
            var reader = new Reader(payload);
            var snapshot = new WorldSnapshot();
            WorldSnapshotGameCodec.Decode(ref reader, ref snapshot);
            return snapshot;
        }

        public static string KindName(byte clientKind) =>
            clientKind < ClientKindNames.Length ? ClientKindNames[clientKind] : "Unknown";

        public static string PlayerLabel(PlayerState state, uint localPlayerId) =>
            $"{KindName(state.ClientKind)} #{state.PlayerId}{(state.PlayerId == localPlayerId ? " (you)" : "")}";
    }
}
