#include "Game/FomoxaExampleNetSubsystem.h"

DEFINE_LOG_CATEGORY_STATIC(LogFomoxaExample, Log, All);

void UFomoxaExampleNetSubsystem::Deinitialize()
{
	NetSession.Close();
	Super::Deinitialize();
}

void UFomoxaExampleNetSubsystem::Tick(float DeltaTime)
{
	NetSession.Poll(fomoxa::now_ms());
	ReportProgress();
}

ETickableTickType UFomoxaExampleNetSubsystem::GetTickableTickType() const
{
	return IsTemplate() ? ETickableTickType::Never : ETickableTickType::Always;
}

bool UFomoxaExampleNetSubsystem::IsTickableWhenPaused() const
{
	return true;
}

TStatId UFomoxaExampleNetSubsystem::GetStatId() const
{
	RETURN_QUICK_DECLARE_CYCLE_STAT(UFomoxaExampleNetSubsystem, STATGROUP_Tickables);
}

void UFomoxaExampleNetSubsystem::Open(const FString& Host, int32 Port, const FString& PlayerName)
{
	UE_LOG(LogFomoxaExample, Log, TEXT("connecting to %s:%d as \"%s\""), *Host, Port, *PlayerName);
	NetSession.Open(TCHAR_TO_UTF8(*Host), static_cast<uint16_t>(FMath::Clamp(Port, 1, 65535)), TCHAR_TO_UTF8(*PlayerName));
	ReportProgress();
}

void UFomoxaExampleNetSubsystem::Close()
{
	NetSession.Close();
	ReportProgress();
}

void UFomoxaExampleNetSubsystem::UpdateInput(float MoveX, float MoveZ, bool bJump, float LookYaw, float LookPitch)
{
	NetSession.UpdateInput(fomoxa::now_ms(), MoveX, MoveZ, bJump, LookYaw, LookPitch);
}

void UFomoxaExampleNetSubsystem::ReportProgress()
{
	const FomoxaExample::SessionStatus CurrentStatus = NetSession.GetStatus();
	if (CurrentStatus != ReportedStatus)
	{
		ReportedStatus = CurrentStatus;
		switch (CurrentStatus)
		{
		case FomoxaExample::SessionStatus::Joining:
			UE_LOG(LogFomoxaExample, Log, TEXT("handshake accepted, joining"));
			break;
		case FomoxaExample::SessionStatus::Joined:
			UE_LOG(LogFomoxaExample, Log, TEXT("joined as player #%u, plane half size %.1f, %u Hz"),
				NetSession.GetLocalPlayerId(), NetSession.GetPlaneHalfSize(), NetSession.GetTickRate());
			break;
		case FomoxaExample::SessionStatus::Failed:
			UE_LOG(LogFomoxaExample, Warning, TEXT("failed: %s"), UTF8_TO_TCHAR(NetSession.GetFailureReason().c_str()));
			break;
		default:
			break;
		}
	}

	const bool bHasSnapshot = NetSession.GetSnapshotsReceived() > 0;
	if (bHasSnapshot && !bReportedSnapshot)
	{
		UE_LOG(LogFomoxaExample, Log, TEXT("first snapshot: tick %lld, %d players"),
			static_cast<long long>(NetSession.GetLastTick()), static_cast<int32>(NetSession.GetPlayers().size()));
	}
	bReportedSnapshot = bHasSnapshot;

	if (!NetSession.GetLastDecodeError().empty())
	{
		UE_LOG(LogFomoxaExample, Warning, TEXT("decode error: %s"), UTF8_TO_TCHAR(NetSession.GetLastDecodeError().c_str()));
	}
}
