using UnrealBuildTool;

public class FomoxaExampleTarget : TargetRules
{
	public FomoxaExampleTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Game;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.Add("FomoxaExample");
	}
}
