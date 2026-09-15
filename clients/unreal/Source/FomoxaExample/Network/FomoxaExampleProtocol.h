#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "fomoxa/net.hpp"
#include "Models/protocol.hpp"

namespace FomoxaExample
{

inline constexpr const char* DefaultHost = "127.0.0.1";
inline constexpr uint16_t DefaultPort = 9321;
inline constexpr uint8_t ClientKindUnreal = 3;

const fmx_schema* Schema();

std::vector<uint8_t> EncodeHello(const std::string& DisplayName);
std::vector<uint8_t> EncodeInput(const FomoxaExampleModels::PlayerInput& Input);

bool DecodeWelcome(fomoxa::ByteView Payload, FomoxaExampleModels::Welcome& OutWelcome, std::string& OutError);
bool DecodeSnapshot(fomoxa::ByteView Payload, FomoxaExampleModels::WorldSnapshot& OutSnapshot, std::string& OutError);

const char* KindName(uint8_t ClientKind);
std::string PlayerLabel(const FomoxaExampleModels::PlayerState& State, uint32_t LocalPlayerId);

}
