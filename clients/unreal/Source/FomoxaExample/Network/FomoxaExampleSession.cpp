#include "Network/FomoxaExampleSession.h"

#include <algorithm>
#include <cstdio>
#include <utility>

#include "Generated/handshake.hpp"
#include "Network/FomoxaExampleProtocol.h"

namespace FomoxaExample
{

Session::~Session()
{
    Close();
}

void Session::Open(const std::string& Host, uint16_t Port, const std::string& PlayerName)
{
    Close();
    FailureReason.clear();

    const std::string Address = Host + ":" + std::to_string(Port);
    std::optional<fomoxa::Transport> Transport = fomoxa::Transport::tcp(Host, Port);
    if (!Transport)
    {
        Fail("cannot reach " + Address);
        return;
    }

    Connection = fomoxa::Connection::create(std::move(*Transport), Schema());
    if (!Connection)
    {
        Fail("cannot start a session with " + Address);
        return;
    }

    DisplayName = PlayerName;
    CurrentStatus = SessionStatus::Connecting;
}

void Session::Close()
{
    Release();
    CurrentStatus = SessionStatus::Disconnected;
}

void Session::Poll(uint64_t NowMs)
{
    if (!Connection)
    {
        return;
    }

    for (fomoxa::Event Raised : Connection->tick(NowMs))
    {
        switch (Raised.kind())
        {
        case FMX_EVENT_READY:
            CurrentStatus = SessionStatus::Joining;
            break;
        case FMX_EVENT_MESSAGE:
            HandleMessage(Raised.message_id(), Raised.payload());
            break;
        case FMX_EVENT_HANDSHAKE_FAILED:
            Fail(std::string("handshake refused: ") + fmx_handshake_failure_name(Raised.handshake_failure()));
            return;
        case FMX_EVENT_DISCONNECTED:
            Fail(std::string("disconnected: ") + fmx_disconnect_name(Raised.disconnect_reason()));
            return;
        default:
            break;
        }
    }

    if (CurrentStatus == SessionStatus::Joining && !bHelloSent)
    {
        const std::vector<uint8_t> Payload = EncodeHello(DisplayName);
        bHelloSent = Connection->send(generated::CLIENT_HELLO_GAME_MESSAGE_ID, fomoxa::ByteView(Payload)) == FMX_OK;
    }
}

void Session::UpdateInput(uint64_t NowMs, float MoveX, float MoveZ, bool bJump, float LookYaw, float LookPitch)
{
    if (CurrentStatus != SessionStatus::Joined || NowMs < NextInputAtMs)
    {
        return;
    }

    FomoxaExampleModels::PlayerInput Input;
    Input.Sequence = ++InputSequence;
    Input.MoveX = MoveX;
    Input.MoveZ = MoveZ;
    Input.Jump = bJump;
    Input.LookYaw = LookYaw;
    Input.LookPitch = LookPitch;

    const std::vector<uint8_t> Payload = EncodeInput(Input);
    if (Connection->send(generated::PLAYER_INPUT_GAME_MESSAGE_ID, fomoxa::ByteView(Payload)) == FMX_OK)
    {
        NextInputAtMs = NowMs + 1000u / std::max<uint32_t>(TickRate, 1u);
    }
}

const FomoxaExampleModels::PlayerState* Session::FindLocalState() const
{
    const auto Found = Players.find(LocalPlayerId);
    return Found == Players.end() ? nullptr : &Found->second;
}

void Session::HandleMessage(uint32_t MessageId, fomoxa::ByteView Payload)
{
    std::string Error;
    if (MessageId == generated::WELCOME_GAME_MESSAGE_ID)
    {
        FomoxaExampleModels::Welcome Welcome;
        if (DecodeWelcome(Payload, Welcome, Error))
        {
            LocalPlayerId = Welcome.PlayerId;
            PlaneHalfSize = Welcome.PlaneHalfSize;
            TickRate = Welcome.TickRate;
            CurrentStatus = SessionStatus::Joined;
        }
    }
    else if (MessageId == generated::WORLD_SNAPSHOT_GAME_MESSAGE_ID)
    {
        FomoxaExampleModels::WorldSnapshot Snapshot;
        if (DecodeSnapshot(Payload, Snapshot, Error))
        {
            LastTick = Snapshot.Tick;
            ++SnapshotsReceived;
            Players.clear();
            for (FomoxaExampleModels::PlayerState& State : Snapshot.Players)
            {
                const uint32_t PlayerId = State.PlayerId;
                Players[PlayerId] = std::move(State);
            }
        }
    }

    if (!Error.empty())
    {
        char MessageLabel[16];
        std::snprintf(MessageLabel, sizeof(MessageLabel), "0x%08X", static_cast<unsigned>(MessageId));
        LastDecodeError = std::string(MessageLabel) + ": " + Error;
    }
}

void Session::Fail(const std::string& Reason)
{
    Release();
    FailureReason = Reason;
    CurrentStatus = SessionStatus::Failed;
}

void Session::Release()
{
    if (Connection)
    {
        Connection->close();
        Connection.reset();
    }
    bHelloSent = false;
    InputSequence = 0;
    NextInputAtMs = 0;
    LocalPlayerId = 0;
    LastTick = -1;
    SnapshotsReceived = 0;
    Players.clear();
}

}
