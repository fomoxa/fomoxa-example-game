import unreal

MAP_PATH = "/Game/Maps/FomoxaExample"
GAME_MODE_CLASS = "/Script/FomoxaExample.FomoxaExampleGameMode"
ENVIRONMENT_CLASS = "/Script/FomoxaExample.FomoxaExampleEnvironment"
FLOOR_CLASS = "/Script/FomoxaExample.FomoxaExampleFloor"
OVERVIEW_LOCATION = unreal.Vector(-1300.0, 0.0, 1600.0)
OVERVIEW_ROTATION = unreal.Rotator(roll=0.0, pitch=-50.906, yaw=0.0)


def load_class(path):
    loaded = unreal.load_class(None, path)
    if loaded is None:
        raise RuntimeError(f"cannot load {path} - build FomoxaExampleEditor first")
    return loaded


def spawn(actor_class, label, location=unreal.Vector(), rotation=unreal.Rotator()):
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    actor = actors.spawn_actor_from_class(actor_class, location, rotation)
    if actor is None:
        raise RuntimeError(f"cannot spawn {label}")
    actor.set_actor_label(label)
    return actor


def main():
    if unreal.EditorAssetLibrary.does_asset_exist(MAP_PATH):
        unreal.EditorAssetLibrary.delete_asset(MAP_PATH)

    world = unreal.EditorLoadingAndSavingUtils.new_blank_map(False)

    spawn(load_class(ENVIRONMENT_CLASS), "Environment")
    spawn(load_class(FLOOR_CLASS), "Floor")
    spawn(unreal.CameraActor, "View Camera", OVERVIEW_LOCATION, OVERVIEW_ROTATION)

    world.get_world_settings().set_editor_property("default_game_mode", load_class(GAME_MODE_CLASS))

    if not unreal.EditorLoadingAndSavingUtils.save_map(world, MAP_PATH):
        raise RuntimeError(f"cannot save {MAP_PATH}")
    unreal.log(f"FOMOXA-EXAMPLE-MAP: saved {MAP_PATH}")


main()
