using System;
using System.Collections.Generic;
using FomoxaExample.Networking.Models;
using Fomoxa.Net;
using Fomoxa.Net.Transports;
using Generated;

namespace FomoxaExample.Networking
{
    public enum FomoxaExampleStatus
    {
        Disconnected,
        Connecting,
        Joining,
        Joined,
        Failed,
    }

    public sealed class FomoxaExampleSession : IDisposable
    {
        readonly Dictionary<uint, PlayerState> players = new Dictionary<uint, PlayerState>();
        FomoxaConnection connection;
        string displayName = "";
        bool helloSent;
        uint inputSequence;
        TimeSpan nextInputAt;

        public FomoxaExampleStatus Status { get; private set; } = FomoxaExampleStatus.Disconnected;
        public string FailureReason { get; private set; } = "";
        public string LastDecodeError { get; private set; } = "";
        public uint LocalPlayerId { get; private set; }
        public float PlaneHalfSize { get; private set; } = 10f;
        public int TickRate { get; private set; } = 30;
        public long LastTick { get; private set; } = -1;
        public int SnapshotsReceived { get; private set; }
        public IReadOnlyDictionary<uint, PlayerState> Players => players;

        public bool TryGetLocalState(out PlayerState state) => players.TryGetValue(LocalPlayerId, out state);

        public void Open(string host, int port, string playerName)
        {
            Close();
            FailureReason = "";
            try
            {
                var transport = TcpTransport.Connect(host, port);
                connection = FomoxaConnection.Connect(transport, FomoxaExampleProtocol.BuildSchema(), new SessionConfig());
            }
            catch (Exception exception)
            {
                Fail($"cannot reach {host}:{port} - {exception.Message}");
                return;
            }
            displayName = playerName;
            Status = FomoxaExampleStatus.Connecting;
        }

        public void Close()
        {
            Release();
            Status = FomoxaExampleStatus.Disconnected;
        }

        public void Dispose() => Close();

        public void Poll() => Poll(MonotonicClock.Now);

        public void Poll(TimeSpan now)
        {
            if (connection == null)
            {
                return;
            }

            var events = connection.Tick(now);
            for (var index = 0; index < events.Count; index++)
            {
                var raised = events[index];
                switch (raised.Kind)
                {
                    case FomoxaEventKind.Ready:
                        Status = FomoxaExampleStatus.Joining;
                        break;
                    case FomoxaEventKind.Message:
                        HandleMessage(raised.MessageId, raised.Payload.Span);
                        break;
                    case FomoxaEventKind.HandshakeFailed:
                        Fail($"handshake refused: {raised.Failure}");
                        return;
                    case FomoxaEventKind.Disconnected:
                        Fail($"disconnected: {raised.Reason}");
                        return;
                }
            }

            if (Status == FomoxaExampleStatus.Joining && !helloSent)
            {
                helloSent = connection.Send(ClientHelloGameCodec.MessageId, FomoxaExampleProtocol.EncodeHello(displayName)) == SendStatus.Sent;
            }
        }

        public void UpdateInput(float moveX, float moveZ, bool jump, float lookYaw, float lookPitch) =>
            UpdateInput(MonotonicClock.Now, moveX, moveZ, jump, lookYaw, lookPitch);

        public void UpdateInput(TimeSpan now, float moveX, float moveZ, bool jump, float lookYaw, float lookPitch)
        {
            if (Status != FomoxaExampleStatus.Joined || now < nextInputAt)
            {
                return;
            }

            inputSequence++;
            var input = new PlayerInput
            {
                Sequence = inputSequence,
                MoveX = moveX,
                MoveZ = moveZ,
                Jump = jump,
                LookYaw = lookYaw,
                LookPitch = lookPitch,
            };
            if (connection.Send(PlayerInputGameCodec.MessageId, FomoxaExampleProtocol.EncodeInput(input)) == SendStatus.Sent)
            {
                nextInputAt = now + TimeSpan.FromSeconds(1.0 / Math.Max(TickRate, 1));
            }
        }

        void HandleMessage(uint messageId, ReadOnlySpan<byte> payload)
        {
            try
            {
                if (messageId == WelcomeGameCodec.MessageId)
                {
                    var welcome = FomoxaExampleProtocol.DecodeWelcome(payload);
                    LocalPlayerId = welcome.PlayerId;
                    PlaneHalfSize = welcome.PlaneHalfSize;
                    TickRate = welcome.TickRate;
                    Status = FomoxaExampleStatus.Joined;
                }
                else if (messageId == WorldSnapshotGameCodec.MessageId)
                {
                    var snapshot = FomoxaExampleProtocol.DecodeSnapshot(payload);
                    LastTick = snapshot.Tick;
                    SnapshotsReceived++;
                    players.Clear();
                    foreach (var state in snapshot.Players)
                    {
                        players[state.PlayerId] = state;
                    }
                }
            }
            catch (DecodeException exception)
            {
                LastDecodeError = $"0x{messageId:X8}: {exception.Message}";
            }
        }

        void Fail(string reason)
        {
            Release();
            FailureReason = reason;
            Status = FomoxaExampleStatus.Failed;
        }

        void Release()
        {
            if (connection != null)
            {
                connection.Close();
                connection.Dispose();
                connection = null;
            }
            helloSent = false;
            inputSequence = 0;
            nextInputAt = TimeSpan.Zero;
            LocalPlayerId = 0;
            LastTick = -1;
            SnapshotsReceived = 0;
            players.Clear();
        }
    }
}
