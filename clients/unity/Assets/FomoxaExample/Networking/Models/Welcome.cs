namespace FomoxaExample.Networking.Models
{
    [Network]
    [Codec("game")]
    public class Welcome
    {
        [Network("u32")]
        [Codec("game")]
        public uint PlayerId { get; set; }

        [Network("f32")]
        [Codec("game")]
        public float PlaneHalfSize { get; set; }

        [Network("u16")]
        [Codec("game")]
        public ushort TickRate { get; set; }
    }
}
