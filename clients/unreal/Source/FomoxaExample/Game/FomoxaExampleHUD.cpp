#include "Game/FomoxaExampleHUD.h"

#include "Engine/Canvas.h"
#include "Engine/Engine.h"

#include "Game/FomoxaExamplePlayerController.h"

namespace FomoxaExampleHUDLayout
{
constexpr float StatusLeft = 16.0f;
constexpr float StatusTop = 12.0f;
constexpr float CrosshairScale = 1.5f;
}

void AFomoxaExampleHUD::DrawHUD()
{
	using namespace FomoxaExampleHUDLayout;

	Super::DrawHUD();

	const AFomoxaExamplePlayerController* ExampleController = Cast<AFomoxaExamplePlayerController>(PlayerOwner);
	if (!ExampleController || !Canvas || !GEngine)
	{
		return;
	}

	UFont* Font = GEngine->GetMediumFont();

	TArray<FString> Lines;
	ExampleController->DescribeStatus().ParseIntoArrayLines(Lines);
	float LineTop = StatusTop;
	for (const FString& Line : Lines)
	{
		float LineWidth = 0.0f;
		float LineHeight = 0.0f;
		GetTextSize(Line, LineWidth, LineHeight, Font);
		DrawText(Line, FLinearColor::White, StatusLeft, LineTop, Font);
		LineTop += LineHeight;
	}

	const FString Crosshair = TEXT("+");
	float CrosshairWidth = 0.0f;
	float CrosshairHeight = 0.0f;
	GetTextSize(Crosshair, CrosshairWidth, CrosshairHeight, Font, CrosshairScale);
	DrawText(Crosshair, FLinearColor::White, (Canvas->ClipX - CrosshairWidth) * 0.5f, (Canvas->ClipY - CrosshairHeight) * 0.5f, Font, CrosshairScale);
}
