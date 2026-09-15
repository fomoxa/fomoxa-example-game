#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"

#include "FomoxaExampleFloor.generated.h"

class UInstancedStaticMeshComponent;
class UMaterialInterface;
class UStaticMeshComponent;

UCLASS()
class FOMOXAEXAMPLE_API AFomoxaExampleFloor : public AActor
{
	GENERATED_BODY()

public:
	AFomoxaExampleFloor();

	virtual void OnConstruction(const FTransform& Transform) override;

	void Rebuild(float NewHalfSize);

protected:
	virtual void BeginPlay() override;

private:
	void Paint();

	UPROPERTY(EditAnywhere, Category = "Fomoxa Example")
	float HalfSize = 10.0f;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UStaticMeshComponent> Ground;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UInstancedStaticMeshComponent> GridLines;

	UPROPERTY(Transient)
	TObjectPtr<UMaterialInterface> ShapeMaterial;

	float BuiltHalfSize = 0.0f;
};
