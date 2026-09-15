#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "fomoxa.h"

namespace FomoxaExampleModels {

FOMOXA_MODEL
FOMOXA_CODEC("game")
struct ClientHello
{
    FOMOXA_FIELD(u8)
    FOMOXA_CODEC("game")
    uint8_t ClientKind = 0;

    FOMOXA_FIELD(string)
    FOMOXA_CODEC("game")
    std::string DisplayName;
};

FOMOXA_MODEL
FOMOXA_CODEC("game")
struct Welcome
{
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("game")
    uint32_t PlayerId = 0;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float PlaneHalfSize = 0.0f;

    FOMOXA_FIELD(u16)
    FOMOXA_CODEC("game")
    uint16_t TickRate = 0;
};

FOMOXA_MODEL
FOMOXA_CODEC("game")
struct PlayerInput
{
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("game")
    uint32_t Sequence = 0;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float MoveX = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float MoveZ = 0.0f;

    FOMOXA_FIELD(bool)
    FOMOXA_CODEC("game")
    bool Jump = false;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float LookYaw = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float LookPitch = 0.0f;
};

FOMOXA_MODEL
FOMOXA_CODEC("game")
struct PlayerState
{
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("game")
    uint32_t PlayerId = 0;

    FOMOXA_FIELD(u8)
    FOMOXA_CODEC("game")
    uint8_t ClientKind = 0;

    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("game")
    uint32_t Color = 0;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float PositionX = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float PositionZ = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float PositionY = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float LookYaw = 0.0f;

    FOMOXA_FIELD(f32)
    FOMOXA_CODEC("game")
    float LookPitch = 0.0f;
};

FOMOXA_MODEL
FOMOXA_CODEC("game")
struct WorldSnapshot
{
    FOMOXA_FIELD(u32)
    FOMOXA_CODEC("game")
    uint32_t Tick = 0;

    FOMOXA_FIELD(Array<PlayerState>)
    FOMOXA_CODEC("game")
    std::vector<PlayerState> Players;
};

}
