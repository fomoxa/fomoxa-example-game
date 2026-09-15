#include "Game/FomoxaExampleAvatar.h"

#include "Components/StaticMeshComponent.h"
#include "Components/TextRenderComponent.h"
#include "Engine/CollisionProfile.h"
#include "Engine/StaticMesh.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"

#include "Game/FomoxaExampleAxes.h"

namespace FomoxaExampleAvatarLayout
{
constexpr float BodyWidth = 0.8f;
constexpr float BodyHeight = 1.2f;
constexpr float HeadWidth = 0.55f;
constexpr float HeadHeight = 0.45f;
constexpr float HeadCenterHeight = 1.45f;
constexpr float VisorWidth = 0.4f;
constexpr float VisorHeight = 0.12f;
constexpr float VisorDepth = 0.08f;
constexpr float VisorLift = 0.05f;
constexpr float LabelHeight = 2.1f;
constexpr float LabelWorldSize = 24.0f;
const FName ColorParameter(TEXT("Color"));
const FColor VisorColor(13, 13, 20);
}

AFomoxaExampleAvatar::AFomoxaExampleAvatar()
{
	using namespace FomoxaExampleAvatarLayout;

	PrimaryActorTick.bCanEverTick = false;

	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(TEXT("/Engine/BasicShapes/Cube.Cube"));
	static ConstructorHelpers::FObjectFinder<UMaterialInterface> BasicShapeMaterial(TEXT("/Engine/BasicShapes/BasicShapeMaterial.BasicShapeMaterial"));
	ShapeMaterial = BasicShapeMaterial.Object;

	RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));
	RootComponent->SetMobility(EComponentMobility::Movable);

	Body = CreateShape(
		TEXT("Body"),
		RootComponent,
		CubeMesh.Object,
		FomoxaExampleAxes::ToUnrealPosition(0.0f, BodyHeight * 0.5f, 0.0f),
		FomoxaExampleAxes::ToUnrealScale(BodyWidth, BodyHeight, BodyWidth));

	Head = CreateDefaultSubobject<USceneComponent>(TEXT("Head"));
	Head->SetupAttachment(RootComponent);
	Head->SetMobility(EComponentMobility::Movable);
	Head->SetRelativeLocation(FomoxaExampleAxes::ToUnrealPosition(0.0f, HeadCenterHeight, 0.0f));

	Skull = CreateShape(TEXT("Skull"), Head, CubeMesh.Object, FVector::ZeroVector, FomoxaExampleAxes::ToUnrealScale(HeadWidth, HeadHeight, HeadWidth));

	Visor = CreateShape(
		TEXT("Visor"),
		Head,
		CubeMesh.Object,
		FomoxaExampleAxes::ToUnrealPosition(0.0f, VisorLift, -(HeadWidth + VisorDepth) * 0.5f),
		FomoxaExampleAxes::ToUnrealScale(VisorWidth, VisorHeight, VisorDepth));

	Label = CreateDefaultSubobject<UTextRenderComponent>(TEXT("Label"));
	Label->SetupAttachment(RootComponent);
	Label->SetMobility(EComponentMobility::Movable);
	Label->SetRelativeLocation(FomoxaExampleAxes::ToUnrealPosition(0.0f, LabelHeight, 0.0f));
	Label->SetUsingAbsoluteRotation(true);
	Label->SetHorizontalAlignment(EHTA_Center);
	Label->SetVerticalAlignment(EVRTA_TextBottom);
	Label->SetWorldSize(LabelWorldSize);
	Label->SetTextRenderColor(FColor::White);
	Label->SetCastShadow(false);
}

void AFomoxaExampleAvatar::Paint(const FLinearColor& Color)
{
	using namespace FomoxaExampleAvatarLayout;

	if (!ShapeMaterial)
	{
		return;
	}

	UMaterialInstanceDynamic* BodyMaterial = UMaterialInstanceDynamic::Create(ShapeMaterial, this);
	BodyMaterial->SetVectorParameterValue(ColorParameter, Color);
	Body->SetMaterial(0, BodyMaterial);
	Skull->SetMaterial(0, BodyMaterial);

	UMaterialInstanceDynamic* VisorMaterial = UMaterialInstanceDynamic::Create(ShapeMaterial, this);
	VisorMaterial->SetVectorParameterValue(ColorParameter, FLinearColor(VisorColor));
	Visor->SetMaterial(0, VisorMaterial);
}

void AFomoxaExampleAvatar::Present(const FVector& Location, const FRotator& BodyRotation, const FRotator& HeadRotation, const FString& LabelText, bool bIsLocalPlayer, const FVector& ViewLocation)
{
	SetActorHiddenInGame(bIsLocalPlayer);
	SetActorLocationAndRotation(Location, BodyRotation);
	Head->SetRelativeRotation(HeadRotation);
	Label->SetText(FText::FromString(LabelText));
	Label->SetWorldRotation((ViewLocation - Label->GetComponentLocation()).Rotation());
}

UStaticMeshComponent* AFomoxaExampleAvatar::CreateShape(FName ShapeName, USceneComponent* Parent, UStaticMesh* Mesh, const FVector& Location, const FVector& Scale)
{
	UStaticMeshComponent* Shape = CreateDefaultSubobject<UStaticMeshComponent>(ShapeName);
	Shape->SetupAttachment(Parent);
	Shape->SetMobility(EComponentMobility::Movable);
	Shape->SetStaticMesh(Mesh);
	Shape->SetRelativeLocation(Location);
	Shape->SetRelativeScale3D(Scale);
	Shape->SetCollisionProfileName(UCollisionProfile::NoCollision_ProfileName);
	return Shape;
}
