using UnityEngine;

namespace FomoxaExample.Game
{
    public static class FomoxaExampleAxes
    {
        public static Vector3 ToUnityPosition(float x, float y, float z) => new Vector3(x, y, -z);

        public static Quaternion ToUnityYaw(float lookYaw) =>
            Quaternion.Euler(0f, -lookYaw * Mathf.Rad2Deg, 0f);

        public static Quaternion ToUnityPitch(float lookPitch) =>
            Quaternion.Euler(-lookPitch * Mathf.Rad2Deg, 0f, 0f);

        public static Quaternion ToUnityLook(float lookYaw, float lookPitch) =>
            Quaternion.Euler(-lookPitch * Mathf.Rad2Deg, -lookYaw * Mathf.Rad2Deg, 0f);

        public static Color32 ToUnityColor(uint rgba) =>
            new Color32((byte)(rgba >> 24), (byte)(rgba >> 16), (byte)(rgba >> 8), (byte)rgba);
    }
}
