namespace FomoxaExample.Networking.Models
{
    [Network]
    [Codec("game")]
    public class ClientHello
    {
        [Network("u8")]
        [Codec("game")]
        public byte ClientKind { get; set; }

        [Network("string")]
        [Codec("game")]
        public string DisplayName { get; set; } = "";
    }
}
