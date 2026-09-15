#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "Tickable.h"

#include "Network/FomoxaExampleSession.h"

#include "FomoxaExampleNetSubsystem.generated.h"

UCLASS()
class FOMOXAEXAMPLE_API UFomoxaExampleNetSubsystem : public UGameInstanceSubsystem, public FTickableGameObject
{
	GENERATED_BODY()

public:
	virtual void Deinitialize() override;

	virtual void Tick(float DeltaTime) override;
	virtual ETickableTickType GetTickableTickType() const override;
	virtual bool IsTickableWhenPaused() const override;
	virtual TStatId GetStatId() const override;

	void Open(const FString& Host, int32 Port, const FString& PlayerName);
	void Close();
	void UpdateInput(float MoveX, float MoveZ, bool bJump, float LookYaw, float LookPitch);

	const FomoxaExample::Session& GetSession() const { return NetSession; }

private:
	void ReportProgress();

	FomoxaExample::Session NetSession;
	FomoxaExample::SessionStatus ReportedStatus = FomoxaExample::SessionStatus::Disconnected;
	bool bReportedSnapshot = false;
};
