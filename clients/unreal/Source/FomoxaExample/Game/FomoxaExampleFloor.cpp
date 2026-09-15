#include "Game/FomoxaExampleFloor.h"

#include "Components/InstancedStaticMeshComponent.h"
#include "Components/StaticMeshComponent.h"
#include "Engine/CollisionProfile.h"
#include "Engine/StaticMesh.h"
#include "Materials/MaterialInstanceDynamic.h"
#include "UObject/ConstructorHelpers.h"

#include "Game/FomoxaExampleAxes.h"

namespace FomoxaExampleFloorLayout
{
constexpr float GridSpacing = 2.0f;
constexpr float GridLineWidth = 0.05f;
constexpr float GridLineHeight = 0.01f;
const FName ColorParameter(TEXT("Color"));
const FColor FloorColor(56, 61, 71);
const FColor GridColor(97, 105, 120);
}

AFomoxaExampleFloor::AFomoxaExampleFloor()
{
	PrimaryActorTick.bCanEverTick = false;

	static ConstructorHelpers::FObjectFinder<UStaticMesh> PlaneMesh(TEXT("/Engine/BasicShapes/Plane.Plane"));
	static ConstructorHelpers::FObjectFinder<UStaticMesh> CubeMesh(TEXT("/Engine/BasicShapes/Cube.Cube"));
	static ConstructorHelpers::FObjectFinder<UMaterialInterface> BasicShapeMaterial(TEXT("/Engine/BasicShapes/BasicShapeMaterial.BasicShapeMaterial"));
	ShapeMaterial = BasicShapeMaterial.Object;

	RootComponent = CreateDefaultSubobject<USceneComponent>(TEXT("Root"));
	RootComponent->SetMobility(EComponentMobility::Movable);

	Ground = CreateDefaultSubobject<UStaticMeshComponent>(TEXT("Ground"));
	Ground->SetupAttachment(RootComponent);
	Ground->SetMobility(EComponentMobility::Movable);
	Ground->SetStaticMesh(PlaneMesh.Object);
	Ground->SetCollisionProfileName(UCollisionProfile::NoCollision_ProfileName);

	GridLines = CreateDefaultSubobject<UInstancedStaticMeshComponent>(TEXT("GridLines"));
	GridLines->SetupAttachment(RootComponent);
	GridLines->SetMobility(EComponentMobility::Movable);
	GridLines->SetStaticMesh(CubeMesh.Object);
	GridLines->SetCollisionProfileName(UCollisionProfile::NoCollision_ProfileName);
	GridLines->SetCastShadow(false);
}

void AFomoxaExampleFloor::OnConstruction(const FTransform& Transform)
{
	Super::OnConstruction(Transform);

	Paint();
	BuiltHalfSize = 0.0f;
	Rebuild(HalfSize);
}

void AFomoxaExampleFloor::BeginPlay()
{
	Super::BeginPlay();

	Paint();
}

void AFomoxaExampleFloor::Rebuild(float NewHalfSize)
{
	using namespace FomoxaExampleFloorLayout;

	if (FMath::IsNearlyEqual(NewHalfSize, BuiltHalfSize))
	{
		return;
	}
	HalfSize = NewHalfSize;
	BuiltHalfSize = NewHalfSize;

	const float Extent = HalfSize * 2.0f;
	Ground->SetRelativeScale3D(FVector(Extent, Extent, 1.0));

	GridLines->ClearInstances();
	const int32 LineCount = FMath::FloorToInt32(Extent / GridSpacing) + 1;
	for (int32 Index = 0; Index < LineCount; ++Index)
	{
		const float Offset = -HalfSize + static_cast<float>(Index) * GridSpacing;
		GridLines->AddInstance(FTransform(
			FRotator::ZeroRotator,
			FomoxaExampleAxes::ToUnrealPosition(Offset, GridLineHeight * 0.5f, 0.0f),
			FomoxaExampleAxes::ToUnrealScale(GridLineWidth, GridLineHeight, Extent)));
		GridLines->AddInstance(FTransform(
			FRotator::ZeroRotator,
			FomoxaExampleAxes::ToUnrealPosition(0.0f, GridLineHeight * 0.5f, Offset),
			FomoxaExampleAxes::ToUnrealScale(Extent, GridLineHeight, GridLineWidth)));
	}
}

void AFomoxaExampleFloor::Paint()
{
	using namespace FomoxaExampleFloorLayout;

	if (!ShapeMaterial)
	{
		return;
	}

	UMaterialInstanceDynamic* GroundMaterial = UMaterialInstanceDynamic::Create(ShapeMaterial, this);
	GroundMaterial->SetVectorParameterValue(ColorParameter, FLinearColor(FloorColor));
	Ground->SetMaterial(0, GroundMaterial);

	UMaterialInstanceDynamic* GridMaterial = UMaterialInstanceDynamic::Create(ShapeMaterial, this);
	GridMaterial->SetVectorParameterValue(ColorParameter, FLinearColor(GridColor));
	GridLines->SetMaterial(0, GridMaterial);
}
