using System;
using FomoxaExample.Game;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using Object = UnityEngine.Object;

namespace FomoxaExample.EditorTools
{
    public static class FomoxaExampleSceneBuilder
    {
        const string RootFolder = "Assets/FomoxaExample";
        const string ScenePath = RootFolder + "/Scenes/Main.unity";
        const string MaterialFolder = RootFolder + "/Materials";
        const float DefaultPlaneHalfSize = 10f;

        static readonly Color BackgroundColor = new Color(0.08f, 0.09f, 0.12f);
        static readonly Color FloorColor = new Color(0.22f, 0.24f, 0.28f);
        static readonly Color GridColor = new Color(0.38f, 0.41f, 0.47f);

        [MenuItem("Fomoxa Example/Rebuild Main Scene")]
        public static void Build()
        {
            EnsureFolder(RootFolder, "Scenes");
            EnsureFolder(RootFolder, "Materials");

            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);

            var viewCamera = new GameObject("Main Camera") { tag = "MainCamera" }.AddComponent<Camera>();
            viewCamera.clearFlags = CameraClearFlags.SolidColor;
            viewCamera.backgroundColor = BackgroundColor;
            viewCamera.nearClipPlane = 0.05f;
            viewCamera.transform.SetPositionAndRotation(FomoxaExampleGame.OverviewPosition, Quaternion.LookRotation(-FomoxaExampleGame.OverviewPosition));
            viewCamera.gameObject.AddComponent<AudioListener>();

            var sun = new GameObject("Directional Light").AddComponent<Light>();
            sun.type = LightType.Directional;
            sun.shadows = LightShadows.Soft;
            sun.transform.rotation = Quaternion.Euler(50f, -30f, 0f);

            var floor = new GameObject("Floor").AddComponent<FomoxaExampleFloor>();
            Assign(floor, "groundMaterial", CreateMaterial("Floor", FloorColor));
            Assign(floor, "gridMaterial", CreateMaterial("Grid", GridColor));
            floor.Rebuild(DefaultPlaneHalfSize);

            var game = new GameObject("Fomoxa Example").AddComponent<FomoxaExampleGame>();
            Assign(game, "viewCamera", viewCamera);
            Assign(game, "floor", floor);

            if (!EditorSceneManager.SaveScene(scene, ScenePath))
            {
                throw new InvalidOperationException($"cannot save {ScenePath}");
            }
            EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(ScenePath, true) };
            AssetDatabase.SaveAssets();
            Debug.Log($"FOMOXA-EXAMPLE-SCENE: saved {ScenePath}");
        }

        static void EnsureFolder(string parent, string folderName)
        {
            if (!AssetDatabase.IsValidFolder($"{parent}/{folderName}"))
            {
                AssetDatabase.CreateFolder(parent, folderName);
            }
        }

        static Material CreateMaterial(string materialName, Color color)
        {
            var path = $"{MaterialFolder}/{materialName}.mat";
            var template = GameObject.CreatePrimitive(PrimitiveType.Cube);
            var material = new Material(template.GetComponent<Renderer>().sharedMaterial) { color = color };
            Object.DestroyImmediate(template);
            AssetDatabase.DeleteAsset(path);
            AssetDatabase.CreateAsset(material, path);
            return material;
        }

        static void Assign(Object target, string propertyName, Object value)
        {
            var serialized = new SerializedObject(target);
            serialized.FindProperty(propertyName).objectReferenceValue = value;
            serialized.ApplyModifiedPropertiesWithoutUndo();
        }
    }
}
