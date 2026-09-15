#include "Game/FomoxaExamplePlayerController.h"

#include "Camera/CameraActor.h"
#include "Camera/CameraComponent.h"
#include "Engine/DirectionalLight.h"
#include "Engine/GameInstance.h"
#include "Engine/LocalPlayer.h"
#include "Engine/World.h"
#include "EngineUtils.h"
#include "EnhancedInputComponent.h"
#include "EnhancedInputSubsystems.h"
#include "InputAction.h"
#include "InputActionValue.h"
#include "InputMappingContext.h"
#include "Misc/CommandLine.h"
#include "Misc/Parse.h"

#include "Game/FomoxaExampleAvatar.h"
#include "Game/FomoxaExampleAxes.h"
#include "Game/FomoxaExampleEnvironment.h"
#include "Game/FomoxaExampleFloor.h"
#include "Game/FomoxaExampleNetSubsystem.h"
#include "Network/FomoxaExampleMath.h"
#include "Network/FomoxaExampleProtocol.h"

namespace FomoxaExampleControllerLayout
{
constexpr float EyeHeight = 1.5f;
constexpr float OverviewX = 0.0f;
constexpr float OverviewY = 16.0f;
constexpr float OverviewZ = 13.0f;
}

AFomoxaExamplePlayerController::AFomoxaExamplePlayerController()
{
	Host = UTF8_TO_TCHAR(FomoxaExample::DefaultHost);
	Port = FomoxaExample::DefaultPort;
	bAutoManageActiveCameraTarget = false;
}

void AFomoxaExamplePlayerController::BeginPlay()
{
	Super::BeginPlay();

	if (!IsLocalController())
	{
		return;
	}

	ResolvedHost = Host;
	ResolvedPort = Port;
	ResolvedName = PlayerName.IsEmpty() ? FString::Printf(TEXT("unreal-%d"), FMath::RandRange(0, 999)) : PlayerName;
	FParse::Value(FCommandLine::Get(), TEXT("host="), ResolvedHost);
	FParse::Value(FCommandLine::Get(), TEXT("port="), ResolvedPort);
	FParse::Value(FCommandLine::Get(), TEXT("name="), ResolvedName);

	if (UEnhancedInputLocalPlayerSubsystem* InputSubsystem = ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(GetLocalPlayer()))
	{
		InputSubsystem->AddMappingContext(LookMapping, 0);
	}

	UWorld* World = GetWorld();
	if (!TActorIterator<ADirectionalLight>(World) && !TActorIterator<AFomoxaExampleEnvironment>(World))
	{
		World->SpawnActor<AFomoxaExampleEnvironment>();
	}

	TActorIterator<AFomoxaExampleFloor> PlacedFloor(World);
	Floor = PlacedFloor ? *PlacedFloor : World->SpawnActor<AFomoxaExampleFloor>();

	TActorIterator<ACameraActor> PlacedCamera(World);
	ViewCamera = PlacedCamera ? *PlacedCamera : World->SpawnActor<ACameraActor>();
	if (ViewCamera)
	{
		ViewCamera->GetCameraComponent()->SetConstraintAspectRatio(false);
		SetViewTarget(ViewCamera);
	}

	ReleaseMouse();
	bConnectPending = true;
}

void AFomoxaExamplePlayerController::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	if (IsLocalController())
	{
		if (UFomoxaExampleNetSubsystem* Net = GetNet())
		{
			Net->Close();
		}
	}
	Super::EndPlay(EndPlayReason);
}

void AFomoxaExamplePlayerController::SetupInputComponent()
{
	Super::SetupInputComponent();

	LookAction = NewObject<UInputAction>(this, TEXT("Look"));
	LookAction->ValueType = EInputActionValueType::Axis2D;

	LookMapping = NewObject<UInputMappingContext>(this, TEXT("LookMapping"));
	LookMapping->MapKey(LookAction, EKeys::Mouse2D);

	if (UEnhancedInputComponent* EnhancedInput = Cast<UEnhancedInputComponent>(InputComponent))
	{
		EnhancedInput->BindAction(LookAction, ETriggerEvent::Triggered, this, &AFomoxaExamplePlayerController::Look);
	}
}

void AFomoxaExamplePlayerController::PlayerTick(float DeltaTime)
{
	Super::PlayerTick(DeltaTime);

	if (!bMouseCaptured && WasInputKeyJustPressed(EKeys::LeftMouseButton))
	{
		CaptureMouse();
	}
	else if (bMouseCaptured && WasInputKeyJustPressed(EKeys::Escape))
	{
		ReleaseMouse();
	}

	UFomoxaExampleNetSubsystem* Net = GetNet();
	if (!Net || !Floor || !ViewCamera)
	{
		return;
	}

	if (bConnectPending)
	{
		bConnectPending = false;
		Connect();
	}

	const float Strafe = KeyAxis(EKeys::A, EKeys::Left, EKeys::D, EKeys::Right);
	const float Forward = KeyAxis(EKeys::S, EKeys::Down, EKeys::W, EKeys::Up);
	const FomoxaExample::FlatMove Move = FomoxaExample::WorldMove(Strafe, Forward, LookYaw);
	Net->UpdateInput(Move.X, Move.Z, IsInputKeyDown(EKeys::SpaceBar), LookYaw, LookPitch);

	Floor->Rebuild(Net->GetSession().GetPlaneHalfSize());
	PlaceCamera(*Net);
	SyncAvatars(*Net);

	if (Net->GetSession().GetStatus() == FomoxaExample::SessionStatus::Failed && WasInputKeyJustPressed(EKeys::R))
	{
		Connect();
	}
}

FString AFomoxaExamplePlayerController::DescribeStatus() const
{
	const UFomoxaExampleNetSubsystem* Net = GetNet();
	if (!Net)
	{
		return TEXT("Disconnected");
	}

	const FomoxaExample::Session& NetSession = Net->GetSession();
	switch (NetSession.GetStatus())
	{
	case FomoxaExample::SessionStatus::Connecting:
		return FString::Printf(TEXT("Connecting to %s:%d..."), *ResolvedHost, ResolvedPort);
	case FomoxaExample::SessionStatus::Joining:
		return TEXT("Handshake accepted, joining...");
	case FomoxaExample::SessionStatus::Joined:
		return FString::Printf(
			TEXT("Player #%u · %d players · tick %lld\nclick: capture mouse · WASD: move · Space: jump · Esc: release mouse (Shift+F1 in PIE)"),
			NetSession.GetLocalPlayerId(),
			static_cast<int32>(NetSession.GetPlayers().size()),
			static_cast<long long>(NetSession.GetLastTick()));
	case FomoxaExample::SessionStatus::Failed:
		return FString::Printf(TEXT("%s · press R to reconnect"), UTF8_TO_TCHAR(NetSession.GetFailureReason().c_str()));
	default:
		return TEXT("Disconnected");
	}
}

UFomoxaExampleNetSubsystem* AFomoxaExamplePlayerController::GetNet() const
{
	return UGameInstance::GetSubsystem<UFomoxaExampleNetSubsystem>(GetGameInstance());
}

void AFomoxaExamplePlayerController::Connect()
{
	if (UFomoxaExampleNetSubsystem* Net = GetNet())
	{
		Net->Open(ResolvedHost, ResolvedPort, ResolvedName);
	}
}

void AFomoxaExamplePlayerController::CaptureMouse()
{
	SetInputMode(FInputModeGameOnly());
	SetShowMouseCursor(false);
	bMouseCaptured = true;
}

void AFomoxaExamplePlayerController::ReleaseMouse()
{
	FInputModeGameAndUI InputMode;
	InputMode.SetHideCursorDuringCapture(false);
	InputMode.SetLockMouseToViewportBehavior(EMouseLockMode::DoNotLock);
	SetInputMode(InputMode);
	SetShowMouseCursor(true);
	bMouseCaptured = false;
}

void AFomoxaExamplePlayerController::Look(const FInputActionValue& Value)
{
	if (!bMouseCaptured)
	{
		return;
	}

	const FVector2D Delta = Value.Get<FVector2D>();
	LookYaw = FomoxaExample::WrapAngle(LookYaw - static_cast<float>(Delta.X) * MouseSensitivity);
	LookPitch = FomoxaExample::ClampPitch(LookPitch + static_cast<float>(Delta.Y) * MouseSensitivity);
}

float AFomoxaExamplePlayerController::KeyAxis(const FKey& Negative, const FKey& NegativeAlternate, const FKey& Positive, const FKey& PositiveAlternate) const
{
	float Value = 0.0f;
	if (IsInputKeyDown(Negative) || IsInputKeyDown(NegativeAlternate))
	{
		Value -= 1.0f;
	}
	if (IsInputKeyDown(Positive) || IsInputKeyDown(PositiveAlternate))
	{
		Value += 1.0f;
	}
	return Value;
}

void AFomoxaExamplePlayerController::PlaceCamera(const UFomoxaExampleNetSubsystem& Net)
{
	using namespace FomoxaExampleControllerLayout;

	const FomoxaExampleModels::PlayerState* Own = Net.GetSession().FindLocalState();
	if (!Own)
	{
		const FVector Overview = FomoxaExampleAxes::ToUnrealPosition(OverviewX, OverviewY, OverviewZ);
		ViewCamera->SetActorLocationAndRotation(Overview, (-Overview).Rotation());
		return;
	}

	ViewCamera->SetActorLocationAndRotation(
		FomoxaExampleAxes::ToUnrealPosition(Own->PositionX, Own->PositionY + EyeHeight, Own->PositionZ),
		FomoxaExampleAxes::ToUnrealLook(LookYaw, LookPitch));
}

void AFomoxaExamplePlayerController::SyncAvatars(const UFomoxaExampleNetSubsystem& Net)
{
	const FomoxaExample::Session& NetSession = Net.GetSession();
	const std::map<uint32_t, FomoxaExampleModels::PlayerState>& Players = NetSession.GetPlayers();

	for (auto Entry = Avatars.CreateIterator(); Entry; ++Entry)
	{
		if (Players.count(Entry.Key()) == 0)
		{
			if (Entry.Value())
			{
				Entry.Value()->Destroy();
			}
			Entry.RemoveCurrent();
		}
	}

	const FVector ViewLocation = ViewCamera->GetActorLocation();
	for (const auto& [PlayerId, State] : Players)
	{
		TObjectPtr<AFomoxaExampleAvatar>& Avatar = Avatars.FindOrAdd(PlayerId);
		if (!Avatar)
		{
			Avatar = GetWorld()->SpawnActor<AFomoxaExampleAvatar>();
			if (!Avatar)
			{
				continue;
			}
			Avatar->Paint(FomoxaExampleAxes::ToUnrealColor(State.Color));
		}

		Avatar->Present(
			FomoxaExampleAxes::ToUnrealPosition(State.PositionX, State.PositionY, State.PositionZ),
			FomoxaExampleAxes::ToUnrealYaw(State.LookYaw),
			FomoxaExampleAxes::ToUnrealPitch(State.LookPitch),
			UTF8_TO_TCHAR(FomoxaExample::PlayerLabel(State, NetSession.GetLocalPlayerId()).c_str()),
			PlayerId == NetSession.GetLocalPlayerId(),
			ViewLocation);
	}
}
