#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>

#include "fomoxa/net.hpp"
#include "Models/protocol.hpp"

namespace FomoxaExample
{

enum class SessionStatus
{
    Disconnected,
    Connecting,
    Joining,
    Joined,
    Failed,
};

class Session
{
public:
    Session() = default;
    Session(const Session&) = delete;
    Session& operator=(const Session&) = delete;
    ~Session();

    void Open(const std::string& Host, uint16_t Port, const std::string& PlayerName);
    void Close();
    void Poll(uint64_t NowMs);
    void UpdateInput(uint64_t NowMs, float MoveX, float MoveZ, bool bJump, float LookYaw, float LookPitch);

    SessionStatus GetStatus() const { return CurrentStatus; }
    const std::string& GetFailureReason() const { return FailureReason; }
    const std::string& GetLastDecodeError() const { return LastDecodeError; }
    uint32_t GetLocalPlayerId() const { return LocalPlayerId; }
    float GetPlaneHalfSize() const { return PlaneHalfSize; }
    uint32_t GetTickRate() const { return TickRate; }
    int64_t GetLastTick() const { return LastTick; }
    uint32_t GetSnapshotsReceived() const { return SnapshotsReceived; }
    const std::map<uint32_t, FomoxaExampleModels::PlayerState>& GetPlayers() const { return Players; }
    const FomoxaExampleModels::PlayerState* FindLocalState() const;

private:
    void HandleMessage(uint32_t MessageId, fomoxa::ByteView Payload);
    void Fail(const std::string& Reason);
    void Release();

    std::optional<fomoxa::Connection> Connection;
    std::map<uint32_t, FomoxaExampleModels::PlayerState> Players;
    SessionStatus CurrentStatus = SessionStatus::Disconnected;
    std::string DisplayName;
    std::string FailureReason;
    std::string LastDecodeError;
    bool bHelloSent = false;
    uint32_t InputSequence = 0;
    uint64_t NextInputAtMs = 0;
    uint32_t LocalPlayerId = 0;
    float PlaneHalfSize = 10.0f;
    uint32_t TickRate = 30;
    int64_t LastTick = -1;
    uint32_t SnapshotsReceived = 0;
};

}
