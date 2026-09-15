namespace FomoxaExample.Networking.Models
{
    [Network]
    [Codec("game")]
    public class PlayerState
    {
        [Network("u32")]
        [Codec("game")]
        public uint PlayerId { get; set; }

        [Network("u8")]
        [Codec("game")]
        public byte ClientKind { get; set; }

        [Network("u32")]
        [Codec("game")]
        public uint Color { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float PositionX { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float PositionZ { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float PositionY { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float LookYaw { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float LookPitch { get; set; }
    }
}
