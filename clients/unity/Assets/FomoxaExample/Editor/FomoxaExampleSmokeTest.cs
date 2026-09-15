using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;
using FomoxaExample.Game;
using FomoxaExample.Networking;
using UnityEditor;
using UnityEngine;
using Debug = UnityEngine.Debug;

namespace FomoxaExample.EditorTools
{
    public static class FomoxaExampleSmokeTest
    {
        const float SentLookYaw = 0.5f;
        const float SentLookPitch = 0.25f;
        const float MinimumJumpHeight = 0.5f;
        const float Tolerance = 1e-3f;

        public static void Run()
        {
            int exitCode;
            try
            {
                exitCode = Execute() ? 0 : 1;
            }
            catch (Exception exception)
            {
                Debug.LogError($"FOMOXA-EXAMPLE-SMOKE: FAIL - {exception}");
                exitCode = 1;
            }
            EditorApplication.Exit(exitCode);
        }

        static bool Execute()
        {
            var failures = new List<string>();
            CheckAxes(failures);

            var options = FomoxaExampleOptions.Parse(Environment.GetCommandLineArgs(), FomoxaExampleProtocol.DefaultHost, FomoxaExampleProtocol.DefaultPort, "unity-smoke");
            using var session = new FomoxaExampleSession();
            session.Open(options.Host, options.Port, options.Name);

            var startX = float.NaN;
            var lastX = float.NaN;
            var highestY = 0f;
            var lastYaw = float.NaN;
            var lastPitch = float.NaN;
            var mostPlayers = 0;
            var clock = Stopwatch.StartNew();
            while (clock.Elapsed.TotalSeconds < options.Seconds && session.Status != FomoxaExampleStatus.Failed)
            {
                session.Poll();
                session.UpdateInput(1f, 0f, true, SentLookYaw, SentLookPitch);
                if (session.TryGetLocalState(out var own))
                {
                    if (float.IsNaN(startX))
                    {
                        startX = own.PositionX;
                    }
                    lastX = own.PositionX;
                    highestY = Math.Max(highestY, own.PositionY);
                    lastYaw = own.LookYaw;
                    lastPitch = own.LookPitch;
                }
                mostPlayers = Math.Max(mostPlayers, session.Players.Count);
                Thread.Sleep(5);
            }

            if (session.Status != FomoxaExampleStatus.Joined)
            {
                failures.Add($"not joined ({session.Status}: {session.FailureReason})");
            }
            if (session.SnapshotsReceived == 0)
            {
                failures.Add("no WorldSnapshot received");
            }
            if (!string.IsNullOrEmpty(session.LastDecodeError))
            {
                failures.Add($"decode error {session.LastDecodeError}");
            }
            if (mostPlayers < options.ExpectPlayers)
            {
                failures.Add($"saw at most {mostPlayers} players, expected {options.ExpectPlayers}");
            }
            if (float.IsNaN(startX) || lastX - startX < 1f)
            {
                failures.Add($"own player did not move along +x (start {startX}, last {lastX})");
            }
            if (highestY < MinimumJumpHeight)
            {
                failures.Add($"own player never jumped above {MinimumJumpHeight} (highest {highestY:F2})");
            }
            if (Math.Abs(lastYaw - SentLookYaw) > Tolerance || Math.Abs(lastPitch - SentLookPitch) > Tolerance)
            {
                failures.Add($"look direction not echoed (yaw {lastYaw}, pitch {lastPitch})");
            }

            var report = $"player #{session.LocalPlayerId} · {session.SnapshotsReceived} snapshots · most players {mostPlayers} · x {startX:F2} -> {lastX:F2} · highest y {highestY:F2} · look ({lastYaw:F2}, {lastPitch:F2})";
            var passed = failures.Count == 0;
            var line = $"FOMOXA-EXAMPLE-SMOKE: {(passed ? "PASS" : "FAIL")} - {(passed ? report : string.Join("; ", failures))}";
            if (passed)
            {
                Debug.Log(line);
            }
            else
            {
                Debug.LogError(line);
            }
            return passed;
        }

        static void CheckAxes(List<string> failures)
        {
            foreach (var yaw in new[] { 0f, 0.5f, -1.2f, 2.8f })
            {
                var protocolForward = new Vector3(-Mathf.Sin(yaw), 0f, -Mathf.Cos(yaw));
                var unityLook = FomoxaExampleAxes.ToUnityLook(yaw, 0f);

                Expect(failures, FomoxaExampleAxes.ToUnityPosition(protocolForward.x, 0f, protocolForward.z), unityLook * Vector3.forward, $"yaw {yaw}: camera forward");

                var forwardMove = FomoxaExampleMath.WorldMove(0f, 1f, yaw);
                Expect(failures, protocolForward, new Vector3(forwardMove.X, 0f, forwardMove.Z), $"yaw {yaw}: W moves along the view");

                var strafeMove = FomoxaExampleMath.WorldMove(1f, 0f, yaw);
                Expect(failures, unityLook * Vector3.right, FomoxaExampleAxes.ToUnityPosition(strafeMove.X, 0f, strafeMove.Z), $"yaw {yaw}: D moves to the view's right");
            }

            var lookingUp = FomoxaExampleAxes.ToUnityLook(0f, 0.5f) * Vector3.forward;
            if (lookingUp.y <= 0f)
            {
                failures.Add($"positive pitch must look up, got {lookingUp}");
            }
        }

        static void Expect(List<string> failures, Vector3 expected, Vector3 actual, string what)
        {
            if ((expected - actual).magnitude > Tolerance)
            {
                failures.Add($"{what}: expected {expected}, got {actual}");
            }
        }
    }
}
