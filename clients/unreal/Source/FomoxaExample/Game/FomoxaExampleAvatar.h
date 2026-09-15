#pragma once

#include "CoreMinimal.h"
#include "GameFramework/Actor.h"

#include "FomoxaExampleAvatar.generated.h"

class UMaterialInterface;
class UStaticMesh;
class UStaticMeshComponent;
class UTextRenderComponent;

UCLASS()
class FOMOXAEXAMPLE_API AFomoxaExampleAvatar : public AActor
{
	GENERATED_BODY()

public:
	AFomoxaExampleAvatar();

	void Paint(const FLinearColor& Color);
	void Present(const FVector& Location, const FRotator& BodyRotation, const FRotator& HeadRotation, const FString& LabelText, bool bIsLocalPlayer, const FVector& ViewLocation);

private:
	UStaticMeshComponent* CreateShape(FName ShapeName, USceneComponent* Parent, UStaticMesh* Mesh, const FVector& Location, const FVector& Scale);

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UStaticMeshComponent> Body;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<USceneComponent> Head;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UStaticMeshComponent> Skull;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UStaticMeshComponent> Visor;

	UPROPERTY(VisibleAnywhere)
	TObjectPtr<UTextRenderComponent> Label;

	UPROPERTY(Transient)
	TObjectPtr<UMaterialInterface> ShapeMaterial;
};
