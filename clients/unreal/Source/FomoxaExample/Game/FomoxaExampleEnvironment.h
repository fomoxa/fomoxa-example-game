#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"

#include "FomoxaExampleEnvironment.generated.h"

class UDirectionalLightComponent;
class USkyAtmosphereComponent;
class USkyLightComponent;

UCLASS()
class FOMOXAEXAMPLE_API AFomoxaExampleEnvironment : public AActor
{
	GENERATED_BODY()

public:
	AFomoxaExampleEnvironment();

private:
	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UDirectionalLightComponent> Sun;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<USkyLightComponent> Sky;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<USkyAtmosphereComponent> Atmosphere;
};
