namespace FomoxaExample.Networking.Models
{
    [Network]
    [Codec("game")]
    public class PlayerInput
    {
        [Network("u32")]
        [Codec("game")]
        public uint Sequence { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float MoveX { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float MoveZ { get; set; }

        [Network("bool")]
        [Codec("game")]
        public bool Jump { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float LookYaw { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float LookPitch { get; set; }
    }
}
