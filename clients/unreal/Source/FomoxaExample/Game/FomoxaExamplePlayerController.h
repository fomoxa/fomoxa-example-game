#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"

#include "FomoxaExamplePlayerController.generated.h"

class ACameraActor;
class AFomoxaExampleAvatar;
class AFomoxaExampleFloor;
class UFomoxaExampleNetSubsystem;
class UInputAction;
class UInputMappingContext;
struct FInputActionValue;
struct FKey;

UCLASS(Config = Game)
class FOMOXAEXAMPLE_API AFomoxaExamplePlayerController : public APlayerController
{
	GENERATED_BODY()

public:
	AFomoxaExamplePlayerController();

	FString DescribeStatus() const;

protected:
	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;
	virtual void SetupInputComponent() override;
	virtual void PlayerTick(float DeltaTime) override;

private:
	UFomoxaExampleNetSubsystem* GetNet() const;
	void Connect();
	void CaptureMouse();
	void ReleaseMouse();
	void Look(const FInputActionValue& Value);
	float KeyAxis(const FKey& Negative, const FKey& NegativeAlternate, const FKey& Positive, const FKey& PositiveAlternate) const;
	void PlaceCamera(const UFomoxaExampleNetSubsystem& Net);
	void SyncAvatars(const UFomoxaExampleNetSubsystem& Net);

	UPROPERTY(Config, EditDefaultsOnly, Category = "Fomoxa Example")
	FString Host;

	UPROPERTY(Config, EditDefaultsOnly, Category = "Fomoxa Example")
	int32 Port = 0;

	UPROPERTY(Config, EditDefaultsOnly, Category = "Fomoxa Example")
	FString PlayerName;

	UPROPERTY(Config, EditDefaultsOnly, Category = "Fomoxa Example")
	float MouseSensitivity = 0.0025f;

	UPROPERTY(Transient)
	TObjectPtr<UInputAction> LookAction;

	UPROPERTY(Transient)
	TObjectPtr<UInputMappingContext> LookMapping;

	UPROPERTY(Transient)
	TObjectPtr<ACameraActor> ViewCamera;

	UPROPERTY(Transient)
	TObjectPtr<AFomoxaExampleFloor> Floor;

	UPROPERTY(Transient)
	TMap<uint32, TObjectPtr<AFomoxaExampleAvatar>> Avatars;

	FString ResolvedHost;
	int32 ResolvedPort = 0;
	FString ResolvedName;
	float LookYaw = 0.0f;
	float LookPitch = 0.0f;
	bool bMouseCaptured = false;
	bool bConnectPending = false;
};
