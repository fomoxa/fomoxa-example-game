#pragma once

#include "CoreMinimal.h"

namespace FomoxaExampleAxes
{

inline constexpr double UnitsPerMeter = 100.0;

inline FVector ToUnrealPosition(float X, float Y, float Z)
{
	return FVector(-Z, X, Y) * UnitsPerMeter;
}

inline FVector ToUnrealScale(float SizeX, float SizeY, float SizeZ)
{
	return FVector(SizeZ, SizeX, SizeY);
}

inline FRotator ToUnrealYaw(float LookYaw)
{
	return FRotator(0.0, -static_cast<double>(FMath::RadiansToDegrees(LookYaw)), 0.0);
}

inline FRotator ToUnrealPitch(float LookPitch)
{
	return FRotator(static_cast<double>(FMath::RadiansToDegrees(LookPitch)), 0.0, 0.0);
}

inline FRotator ToUnrealLook(float LookYaw, float LookPitch)
{
	return FRotator(static_cast<double>(FMath::RadiansToDegrees(LookPitch)), -static_cast<double>(FMath::RadiansToDegrees(LookYaw)), 0.0);
}

inline FLinearColor ToUnrealColor(uint32 Rgba)
{
	return FLinearColor(FColor(static_cast<uint8>(Rgba >> 24), static_cast<uint8>(Rgba >> 16), static_cast<uint8>(Rgba >> 8), static_cast<uint8>(Rgba)));
}

}
