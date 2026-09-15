#pragma once

#include "CoreMinimal.h"
#include "GameFramework/HUD.h"

#include "FomoxaExampleHUD.generated.h"

UCLASS()
class FOMOXAEXAMPLE_API AFomoxaExampleHUD : public AHUD
{
	GENERATED_BODY()

public:
	virtual void DrawHUD() override;
};
