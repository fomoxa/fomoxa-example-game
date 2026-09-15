#include "Game/FomoxaExampleGameMode.h"

#include "Game/FomoxaExampleHUD.h"
#include "Game/FomoxaExamplePlayerController.h"

AFomoxaExampleGameMode::AFomoxaExampleGameMode()
{
	PlayerControllerClass = AFomoxaExamplePlayerController::StaticClass();
	HUDClass = AFomoxaExampleHUD::StaticClass();
	DefaultPawnClass = nullptr;
}

void AFomoxaExampleGameMode::HandleStartingNewPlayer_Implementation(APlayerController* NewPlayer)
{
}
