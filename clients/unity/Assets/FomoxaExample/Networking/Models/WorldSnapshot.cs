using System.Collections.Generic;

namespace FomoxaExample.Networking.Models
{
    [Network]
    [Codec("game")]
    public class WorldSnapshot
    {
        [Network("u32")]
        [Codec("game")]
        public uint Tick { get; set; }

        [Network("Array<PlayerState>")]
        [Codec("game")]
        public List<PlayerState> Players { get; set; } = new List<PlayerState>();
    }
}
