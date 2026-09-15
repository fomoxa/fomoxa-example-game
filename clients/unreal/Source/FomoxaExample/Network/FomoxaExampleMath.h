#pragma once

#include <algorithm>
#include <cmath>

namespace FomoxaExample
{

inline constexpr float MaxLookPitch = 1.5f;
inline constexpr float HalfTurn = 3.14159265358979323846f;

struct FlatMove
{
    float X = 0.0f;
    float Z = 0.0f;
};

inline FlatMove WorldMove(float Strafe, float Forward, float LookYaw)
{
    const float YawSin = std::sin(LookYaw);
    const float YawCos = std::cos(LookYaw);
    return {YawCos * Strafe - YawSin * Forward, -YawSin * Strafe - YawCos * Forward};
}

inline float WrapAngle(float Angle)
{
    float Wrapped = std::fmod(Angle + HalfTurn, 2.0f * HalfTurn);
    if (Wrapped < 0.0f)
    {
        Wrapped += 2.0f * HalfTurn;
    }
    return Wrapped - HalfTurn;
}

inline float ClampPitch(float Pitch)
{
    return std::clamp(Pitch, -MaxLookPitch, MaxLookPitch);
}

}
