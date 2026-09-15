using System;
using System.Globalization;

namespace FomoxaExample.Networking
{
    public sealed class FomoxaExampleOptions
    {
        public string Host { get; private set; }
        public int Port { get; private set; }
        public string Name { get; private set; }
        public float Seconds { get; private set; } = 4f;
        public int ExpectPlayers { get; private set; } = 1;

        public static FomoxaExampleOptions Parse(string[] arguments, string defaultHost, int defaultPort, string defaultName)
        {
            var options = new FomoxaExampleOptions { Host = defaultHost, Port = defaultPort, Name = defaultName };
            foreach (var argument in arguments)
            {
                var separator = argument.IndexOf('=');
                if (!argument.StartsWith("--", StringComparison.Ordinal) || separator < 0)
                {
                    continue;
                }

                var key = argument.Substring(2, separator - 2);
                var value = argument.Substring(separator + 1);
                switch (key)
                {
                    case "host":
                        options.Host = value;
                        break;
                    case "port" when int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var port):
                        options.Port = port;
                        break;
                    case "name":
                        options.Name = value;
                        break;
                    case "seconds" when float.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out var seconds):
                        options.Seconds = seconds;
                        break;
                    case "expect-players" when int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out var expected):
                        options.ExpectPlayers = expected;
                        break;
                }
            }
            return options;
        }
    }
}
