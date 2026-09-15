#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"

#include "FomoxaExampleGameMode.generated.h"

UCLASS()
class FOMOXAEXAMPLE_API AFomoxaExampleGameMode : public AGameModeBase
{
	GENERATED_BODY()

public:
	AFomoxaExampleGameMode();

	virtual void HandleStartingNewPlayer_Implementation(APlayerController* NewPlayer) override;
};
