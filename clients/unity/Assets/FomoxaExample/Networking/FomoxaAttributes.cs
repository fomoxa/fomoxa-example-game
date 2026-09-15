using System;

[AttributeUsage(AttributeTargets.Class | AttributeTargets.Property)]
public sealed class NetworkAttribute : Attribute
{
    public NetworkAttribute() { }
    public NetworkAttribute(string wireType) { }
}

[AttributeUsage(AttributeTargets.Class | AttributeTargets.Property)]
public sealed class CodecAttribute : Attribute
{
    public CodecAttribute(params string[] codecs) { }
}
