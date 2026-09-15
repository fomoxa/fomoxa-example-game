using UnrealBuildTool;

public class FomoxaExampleEditorTarget : TargetRules
{
	public FomoxaExampleEditorTarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Editor;
		DefaultBuildSettings = BuildSettingsVersion.Latest;
		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
		ExtraModuleNames.Add("FomoxaExample");
	}
}
