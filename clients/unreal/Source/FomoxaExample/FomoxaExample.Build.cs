using System.IO;
using UnrealBuildTool;

public class FomoxaExample : ModuleRules
{
	public FomoxaExample(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PublicDependencyModuleNames.AddRange(new[] { "Core", "CoreUObject", "Engine", "InputCore", "EnhancedInput" });

		PrivateIncludePaths.AddRange(new[]
		{
			ModuleDirectory,
			Path.Combine(ModuleDirectory, "Protocol"),
			Path.Combine(ModuleDirectory, "ThirdParty", "fomoxa", "include"),
		});

		CppCompileWarningSettings.ShadowVariableWarningLevel = WarningLevel.Warning;
		CppCompileWarningSettings.UndefinedIdentifierWarningLevel = WarningLevel.Off;

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			PublicSystemLibraries.Add("ws2_32.lib");
		}
	}
}
