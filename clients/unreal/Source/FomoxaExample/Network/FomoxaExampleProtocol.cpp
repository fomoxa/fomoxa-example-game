#include "Network/FomoxaExampleProtocol.h"

#include <iterator>

#include "Generated/client_hello_game.hpp"
#include "Generated/handshake.hpp"
#include "Generated/player_input_game.hpp"
#include "Generated/welcome_game.hpp"
#include "Generated/world_snapshot_game.hpp"

namespace FomoxaExample
{

const fmx_schema* Schema()
{
    static const std::vector<fmx_message_schema> Messages = []
    {
        std::vector<fmx_message_schema> Built;
        Built.reserve(generated::FOMOXA_MESSAGES.size());
        for (const generated::FomoxaMessage& Message : generated::FOMOXA_MESSAGES)
        {
            fmx_message_schema Entry{};
            Entry.id = Message.id;
            Entry.fingerprint = Message.fingerprint;
            Entry.prefixes = Message.prefixes;
            Entry.prefix_count = Message.prefix_count;
            Built.push_back(Entry);
        }
        return Built;
    }();

    static const fmx_schema Built = []
    {
        fmx_schema Table{};
        Table.fingerprint = generated::FOMOXA_SCHEMA_FINGERPRINT;
        Table.messages = Messages.data();
        Table.message_count = Messages.size();
        return Table;
    }();

    return &Built;
}

std::vector<uint8_t> EncodeHello(const std::string& DisplayName)
{
    FomoxaExampleModels::ClientHello Hello;
    Hello.ClientKind = ClientKindUnreal;
    Hello.DisplayName = DisplayName;
    generated::Writer PayloadWriter;
    generated::ClientHelloGameCodec::encode(PayloadWriter, Hello);
    return PayloadWriter.into_bytes();
}

std::vector<uint8_t> EncodeInput(const FomoxaExampleModels::PlayerInput& Input)
{
    generated::Writer PayloadWriter;
    generated::PlayerInputGameCodec::encode(PayloadWriter, Input);
    return PayloadWriter.into_bytes();
}

bool DecodeWelcome(fomoxa::ByteView Payload, FomoxaExampleModels::Welcome& OutWelcome, std::string& OutError)
{
    generated::Reader PayloadReader(Payload.data(), Payload.size());
    const generated::DecodeError Error = generated::WelcomeGameCodec::decode(PayloadReader, OutWelcome);
    OutError = Error.ok() ? std::string() : Error.message();
    return Error.ok();
}

bool DecodeSnapshot(fomoxa::ByteView Payload, FomoxaExampleModels::WorldSnapshot& OutSnapshot, std::string& OutError)
{
    generated::Reader PayloadReader(Payload.data(), Payload.size());
    const generated::DecodeError Error = generated::WorldSnapshotGameCodec::decode(PayloadReader, OutSnapshot);
    OutError = Error.ok() ? std::string() : Error.message();
    return Error.ok();
}

const char* KindName(uint8_t ClientKind)
{
    static constexpr const char* Names[] = {"Bot", "Unity", "Godot", "Unreal", "Kaiju", "nunuStudio"};
    return ClientKind < std::size(Names) ? Names[ClientKind] : "Unknown";
}

std::string PlayerLabel(const FomoxaExampleModels::PlayerState& State, uint32_t LocalPlayerId)
{
    std::string Label = std::string(KindName(State.ClientKind)) + " #" + std::to_string(State.PlayerId);
    if (State.PlayerId == LocalPlayerId)
    {
        Label += " (you)";
    }
    return Label;
}

}
