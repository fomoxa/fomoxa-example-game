using UnityEngine;

namespace FomoxaExample.Game
{
    public sealed class FomoxaExampleFloor : MonoBehaviour
    {
        const float GridSpacing = 2f;
        const float GridLineWidth = 0.05f;
        const float GridLineHeight = 0.01f;
        const float UnityPlaneSize = 10f;

        [SerializeField] Material groundMaterial;
        [SerializeField] Material gridMaterial;
        [SerializeField] float halfSize;

        public void Rebuild(float newHalfSize)
        {
            if (Mathf.Approximately(newHalfSize, halfSize) && transform.childCount > 0)
            {
                return;
            }

            halfSize = newHalfSize;
            for (var index = transform.childCount - 1; index >= 0; index--)
            {
                Discard(transform.GetChild(index).gameObject);
            }

            var extent = halfSize * 2f;
            var ground = CreateShape(PrimitiveType.Plane, "Ground", groundMaterial);
            ground.transform.localScale = Vector3.one * (extent / UnityPlaneSize);

            var lineCount = Mathf.FloorToInt(extent / GridSpacing) + 1;
            for (var index = 0; index < lineCount; index++)
            {
                var offset = -halfSize + index * GridSpacing;
                PlaceBox(CreateShape(PrimitiveType.Cube, "Grid X", gridMaterial), new Vector3(offset, GridLineHeight * 0.5f, 0f), new Vector3(GridLineWidth, GridLineHeight, extent));
                PlaceBox(CreateShape(PrimitiveType.Cube, "Grid Z", gridMaterial), new Vector3(0f, GridLineHeight * 0.5f, offset), new Vector3(extent, GridLineHeight, GridLineWidth));
            }
        }

        GameObject CreateShape(PrimitiveType type, string shapeName, Material material)
        {
            var shape = GameObject.CreatePrimitive(type);
            shape.name = shapeName;
            Discard(shape.GetComponent<Collider>());
            shape.transform.SetParent(transform, false);
            shape.GetComponent<Renderer>().sharedMaterial = material;
            return shape;
        }

        static void PlaceBox(GameObject box, Vector3 localPosition, Vector3 size)
        {
            box.transform.localPosition = localPosition;
            box.transform.localScale = size;
        }

        static void Discard(Object target)
        {
            if (Application.isPlaying)
            {
                Destroy(target);
            }
            else
            {
                DestroyImmediate(target);
            }
        }
    }
}
