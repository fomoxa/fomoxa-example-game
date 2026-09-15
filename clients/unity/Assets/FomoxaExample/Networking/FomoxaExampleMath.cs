using System;

namespace FomoxaExample.Networking
{
    public static class FomoxaExampleMath
    {
        const float FullTurn = 2f * MathF.PI;

        public static (float X, float Z) WorldMove(float strafe, float forward, float lookYaw)
        {
            var sin = MathF.Sin(lookYaw);
            var cos = MathF.Cos(lookYaw);
            return (cos * strafe - sin * forward, -sin * strafe - cos * forward);
        }

        public static float WrapAngle(float angle)
        {
            var wrapped = (angle + MathF.PI) % FullTurn;
            if (wrapped < 0f)
            {
                wrapped += FullTurn;
            }
            return wrapped - MathF.PI;
        }

        public static float ClampPitch(float pitch) =>
            Math.Clamp(pitch, -FomoxaExampleProtocol.MaxLookPitch, FomoxaExampleProtocol.MaxLookPitch);
    }
}
