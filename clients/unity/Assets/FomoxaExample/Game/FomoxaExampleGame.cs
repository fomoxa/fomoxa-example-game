using System;
using System.Collections.Generic;
using FomoxaExample.Networking;
using FomoxaExample.Networking.Models;
using UnityEngine;

namespace FomoxaExample.Game
{
    public sealed class FomoxaExampleGame : MonoBehaviour
    {
        public static readonly Vector3 OverviewPosition = new Vector3(0f, 16f, -13f);

        const float EyeHeight = 1.5f;
        const float HeadCenterHeight = 1.45f;
        const float LabelHeight = 2.1f;

        static readonly Vector3 BodySize = new Vector3(0.8f, 1.2f, 0.8f);
        static readonly Vector3 HeadSize = new Vector3(0.55f, 0.45f, 0.55f);
        static readonly Vector3 VisorSize = new Vector3(0.4f, 0.12f, 0.08f);
        static readonly Color VisorColor = new Color(0.05f, 0.05f, 0.08f);

        [SerializeField] string host = FomoxaExampleProtocol.DefaultHost;
        [SerializeField] int port = FomoxaExampleProtocol.DefaultPort;
        [SerializeField] string playerName = "";
        [SerializeField] float mouseSensitivity = 0.05f;
        [SerializeField] Camera viewCamera;
        [SerializeField] FomoxaExampleFloor floor;

        readonly FomoxaExampleSession session = new FomoxaExampleSession();
        readonly Dictionary<uint, AvatarView> avatars = new Dictionary<uint, AvatarView>();
        readonly List<uint> departedPlayers = new List<uint>();
        FomoxaExampleOptions options;
        float lookYaw;
        float lookPitch;
        Font labelFont;
        GUIStyle crosshairStyle;

        sealed class AvatarView
        {
            public GameObject Root;
            public Transform Head;
            public TextMesh Label;
        }

        void Start()
        {
            Application.runInBackground = true;
            if (viewCamera == null)
            {
                viewCamera = Camera.main;
            }
            var defaultName = string.IsNullOrEmpty(playerName) ? $"unity-{UnityEngine.Random.Range(0, 1000)}" : playerName;
            options = FomoxaExampleOptions.Parse(Environment.GetCommandLineArgs(), host, port, defaultName);
            labelFont = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf");
            Connect();
        }

        void Update()
        {
            ReadLook();
            session.Poll();

            var strafe = Axis(KeyCode.A, KeyCode.LeftArrow, KeyCode.D, KeyCode.RightArrow);
            var forward = Axis(KeyCode.S, KeyCode.DownArrow, KeyCode.W, KeyCode.UpArrow);
            var move = FomoxaExampleMath.WorldMove(strafe, forward, lookYaw);
            session.UpdateInput(move.X, move.Z, Input.GetKey(KeyCode.Space), lookYaw, lookPitch);

            if (floor != null)
            {
                floor.Rebuild(session.PlaneHalfSize);
            }
            if (viewCamera != null)
            {
                PlaceCamera();
                SyncAvatars();
            }

            if (session.Status == FomoxaExampleStatus.Failed && Input.GetKeyDown(KeyCode.R))
            {
                Connect();
            }
        }

        void OnDestroy() => session.Dispose();

        void OnGUI()
        {
            GUI.Label(new Rect(16f, 12f, 900f, 60f), StatusText());
            crosshairStyle ??= new GUIStyle(GUI.skin.label) { alignment = TextAnchor.MiddleCenter, fontSize = 20 };
            GUI.Label(new Rect(0f, 0f, Screen.width, Screen.height), "+", crosshairStyle);
        }

        void Connect() => session.Open(options.Host, options.Port, options.Name);

        void ReadLook()
        {
            if (Input.GetMouseButtonDown(0))
            {
                Cursor.lockState = CursorLockMode.Locked;
                Cursor.visible = false;
            }
            if (Input.GetKeyDown(KeyCode.Escape))
            {
                Cursor.lockState = CursorLockMode.None;
                Cursor.visible = true;
            }
            if (Cursor.lockState != CursorLockMode.Locked)
            {
                return;
            }

            lookYaw = FomoxaExampleMath.WrapAngle(lookYaw - Input.GetAxis("Mouse X") * mouseSensitivity);
            lookPitch = FomoxaExampleMath.ClampPitch(lookPitch + Input.GetAxis("Mouse Y") * mouseSensitivity);
        }

        static float Axis(KeyCode negative, KeyCode negativeAlternate, KeyCode positive, KeyCode positiveAlternate)
        {
            var value = 0f;
            if (Input.GetKey(negative) || Input.GetKey(negativeAlternate))
            {
                value -= 1f;
            }
            if (Input.GetKey(positive) || Input.GetKey(positiveAlternate))
            {
                value += 1f;
            }
            return value;
        }

        void PlaceCamera()
        {
            if (!session.TryGetLocalState(out var own))
            {
                viewCamera.transform.SetPositionAndRotation(OverviewPosition, Quaternion.LookRotation(-OverviewPosition));
                return;
            }
            viewCamera.transform.SetPositionAndRotation(
                FomoxaExampleAxes.ToUnityPosition(own.PositionX, own.PositionY + EyeHeight, own.PositionZ),
                FomoxaExampleAxes.ToUnityLook(lookYaw, lookPitch));
        }

        void SyncAvatars()
        {
            departedPlayers.Clear();
            foreach (var playerId in avatars.Keys)
            {
                if (!session.Players.ContainsKey(playerId))
                {
                    departedPlayers.Add(playerId);
                }
            }
            foreach (var playerId in departedPlayers)
            {
                Destroy(avatars[playerId].Root);
                avatars.Remove(playerId);
            }

            foreach (var state in session.Players.Values)
            {
                if (!avatars.TryGetValue(state.PlayerId, out var avatar))
                {
                    avatar = CreateAvatar(state);
                    avatars.Add(state.PlayerId, avatar);
                }
                avatar.Root.SetActive(state.PlayerId != session.LocalPlayerId);
                avatar.Root.transform.SetPositionAndRotation(
                    FomoxaExampleAxes.ToUnityPosition(state.PositionX, state.PositionY, state.PositionZ),
                    FomoxaExampleAxes.ToUnityYaw(state.LookYaw));
                avatar.Head.localRotation = FomoxaExampleAxes.ToUnityPitch(state.LookPitch);
                avatar.Label.text = FomoxaExampleProtocol.PlayerLabel(state, session.LocalPlayerId);
                avatar.Label.transform.rotation = Quaternion.LookRotation(avatar.Label.transform.position - viewCamera.transform.position);
            }
        }

        AvatarView CreateAvatar(PlayerState state)
        {
            Color color = FomoxaExampleAxes.ToUnityColor(state.Color);
            var root = new GameObject($"Player {state.PlayerId}");
            PlaceBox(CreateShape(PrimitiveType.Cube, "Body", root.transform, color), new Vector3(0f, BodySize.y * 0.5f, 0f), BodySize);

            var head = new GameObject("Head").transform;
            head.SetParent(root.transform, false);
            head.localPosition = new Vector3(0f, HeadCenterHeight, 0f);
            PlaceBox(CreateShape(PrimitiveType.Cube, "Skull", head, color), Vector3.zero, HeadSize);
            PlaceBox(CreateShape(PrimitiveType.Cube, "Visor", head, VisorColor), new Vector3(0f, 0.05f, (HeadSize.z + VisorSize.z) * 0.5f), VisorSize);

            var labelObject = new GameObject("Label");
            labelObject.transform.SetParent(root.transform, false);
            labelObject.transform.localPosition = new Vector3(0f, LabelHeight, 0f);
            var label = labelObject.AddComponent<TextMesh>();
            label.font = labelFont;
            labelObject.GetComponent<MeshRenderer>().sharedMaterial = labelFont.material;
            label.fontSize = 64;
            label.characterSize = 0.05f;
            label.anchor = TextAnchor.LowerCenter;
            label.alignment = TextAlignment.Center;
            label.color = Color.white;

            return new AvatarView { Root = root, Head = head, Label = label };
        }

        static GameObject CreateShape(PrimitiveType type, string shapeName, Transform parent, Color color)
        {
            var shape = GameObject.CreatePrimitive(type);
            shape.name = shapeName;
            Destroy(shape.GetComponent<Collider>());
            shape.transform.SetParent(parent, false);
            shape.GetComponent<Renderer>().material.color = color;
            return shape;
        }

        static void PlaceBox(GameObject box, Vector3 localPosition, Vector3 size)
        {
            box.transform.localPosition = localPosition;
            box.transform.localScale = size;
        }

        string StatusText()
        {
            switch (session.Status)
            {
                case FomoxaExampleStatus.Connecting:
                    return $"Connecting to {options.Host}:{options.Port}...";
                case FomoxaExampleStatus.Joining:
                    return "Handshake accepted, joining...";
                case FomoxaExampleStatus.Joined:
                    return $"Player #{session.LocalPlayerId} · {session.Players.Count} players · tick {session.LastTick}\nclick: capture mouse · WASD: move · Space: jump · Esc: release mouse";
                case FomoxaExampleStatus.Failed:
                    return $"{session.FailureReason} · press R to reconnect";
                default:
                    return "Disconnected";
            }
        }
    }
}
