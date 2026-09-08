use haven_assets::asset_intake::repo_root_dir;
use haven_core::{GameWorld, ObjectId, ObjectKind, ProjectSceneId};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

pub(crate) const DEVELOPMENT_WORLD_DESCRIPTOR: &str = "WORKSPACE/development/active_world.json";

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct DevelopmentWorldDescriptor {
    pub(crate) schema: String,
    pub(crate) world_id: String,
    pub(crate) character_id: String,
    pub(crate) default_scene: String,
    pub(crate) spawn: DevelopmentSpawn,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) struct DevelopmentSpawn {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl DevelopmentWorldDescriptor {
    pub(crate) fn load() -> Result<Self, String> {
        let root = repo_root_dir();
        let path = root.join(DEVELOPMENT_WORLD_DESCRIPTOR);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let descriptor: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if descriptor.schema != "havenwild.development_world.v1" {
            return Err(format!("unsupported development descriptor schema {}", descriptor.schema));
        }
        if descriptor.world_id.trim().is_empty() || descriptor.character_id.trim().is_empty() {
            return Err("development descriptor requires world_id and character_id".to_string());
        }
        Ok(descriptor)
    }

    pub(crate) fn world_path(&self) -> PathBuf {
        repo_root_dir()
            .join("WORKSPACE/saves")
            .join(&self.world_id)
            .join("world.tworld")
    }
}

pub(crate) fn editor_world_path() -> PathBuf {
    DevelopmentWorldDescriptor::load()
        .map(|descriptor| descriptor.world_path())
        .unwrap_or_else(|_| repo_root_dir().join("WORKSPACE/saves/world.tworld"))
}

pub(crate) fn launch(
    descriptor: &DevelopmentWorldDescriptor,
    scene: Option<&str>,
    spawn: Option<[i32; 2]>,
) -> Result<Child, String> {
    let root = repo_root_dir();
    let mut args = vec![
        "--dev-world".to_string(),
        descriptor.world_id.clone(),
        "--dev-character".to_string(),
        descriptor.character_id.clone(),
    ];
    if let Some(scene) = scene {
        args.push("--dev-scene".to_string());
        args.push(scene.to_string());
    }
    if let Some([x, y]) = spawn {
        args.extend([
            "--dev-x".to_string(),
            x.to_string(),
            "--dev-y".to_string(),
            y.to_string(),
        ]);
    }

    if let Some(executable) = sibling_game_executable() {
        let mut command = Command::new(&executable);
        command
            .args(&args)
            .current_dir(&root)
            // Native-editor Play must consume the exact world.tworld published
            // by the editor, never the terrain-certification test-world pack.
            .env("HAVENWILD_CLIENT_WORLDGEN_TEST", "0");
        attach_development_client_stdio(&mut command, &root);
        return command
            .spawn()
            .map_err(|error| format!("{}: {error}", executable.display()));
    }

    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .env("HAVENWILD_CLIENT_WORLDGEN_TEST", "0")
        .args(["run", "-p", "haven_game", "--"])
        .args(&args);
    attach_development_client_stdio(&mut command, &root);
    command
        .spawn()
        .map_err(|error| format!("unable to launch haven_game through Cargo: {error}"))
}

fn attach_development_client_stdio(command: &mut Command, root: &Path) {
    let logs = root.join("logs");
    if std::fs::create_dir_all(&logs).is_err() {
        return;
    }
    let path = logs.join("haven_development_client_stdio.log");
    let Ok(stderr_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    if let Ok(stdout_file) = stderr_file.try_clone() {
        command.stdout(Stdio::from(stdout_file));
    }
    command.stderr(Stdio::from(stderr_file));
}

fn sibling_game_executable() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let directory = current.parent()?;
    let candidate = directory.join(if cfg!(windows) { "haven_game.exe" } else { "haven_game" });
    candidate.is_file().then_some(candidate)
}

pub(crate) fn stop(child: &mut Child) -> Result<(), String> {
    match child.try_wait().map_err(|error| error.to_string())? {
        Some(_) => Ok(()),
        None => child.kill().map_err(|error| error.to_string()),
    }
}

pub(crate) fn path_label(path: &Path) -> String {
    path.strip_prefix(repo_root_dir())
        .unwrap_or(path)
        .display()
        .to_string()
}


pub(crate) const DEVELOPMENT_ACCEPTANCE_CRATE_OFFSET: [i32; 2] = [3, 0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DevelopmentAcceptanceObject {
    pub(crate) id: ObjectId,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// Ensures the canonical development world contains one ordinary persistent crate
/// close to the configured spawn. This is intentionally a normal authored object:
/// moving it in the editor exercises the same object/save/runtime path used by game content.
pub(crate) fn ensure_acceptance_crate(
    world: &mut GameWorld,
    descriptor: &DevelopmentWorldDescriptor,
) -> Result<DevelopmentAcceptanceObject, String> {
    let scene_id = ProjectSceneId::new(descriptor.default_scene.clone());
    let scene = world
        .scene_mut(&scene_id)
        .ok_or_else(|| format!("development scene {} is missing", scene_id))?;
    // The scene owns its canonical spawn. The development descriptor identifies the
    // scene but must not become a second spawn authority that can drift when the
    // selected/default scene changes.
    let preferred_x = scene.spawn_x + DEVELOPMENT_ACCEPTANCE_CRATE_OFFSET[0];
    let preferred_y = scene.spawn_y + DEVELOPMENT_ACCEPTANCE_CRATE_OFFSET[1];

    if let Some(object) = scene
        .map
        .objects
        .iter()
        .find(|object| object.kind == ObjectKind::Crate && object.x == preferred_x && object.y == preferred_y)
        .copied()
    {
        return Ok(DevelopmentAcceptanceObject { id: object.id, x: object.x, y: object.y });
    }

    let id = scene
        .map
        .place_object(ObjectKind::Crate, preferred_x, preferred_y)
        .or_else(|| {
            // Keep the fixture deterministic while tolerating generated content near spawn.
            (1..=8).find_map(|distance| {
                scene.map.place_object(ObjectKind::Crate, preferred_x + distance, preferred_y)
            })
        })
        .ok_or_else(|| "unable to place development acceptance crate near spawn".to_string())?;
    let object = scene
        .map
        .object(id)
        .copied()
        .ok_or_else(|| "development acceptance crate disappeared after placement".to_string())?;
    Ok(DevelopmentAcceptanceObject { id, x: object.x, y: object.y })
}

#[derive(serde::Serialize)]
struct LiveCommandEnvelope<'a, T: serde::Serialize> {
    schema: &'static str,
    world_id: &'a str,
    sequence: u64,
    command: T,
}

#[derive(serde::Serialize)]
struct LiveMoveObjectCommand<'a> {
    #[serde(rename = "type")]
    command_type: &'static str,
    scene_id: &'a str,
    object_id: u64,
    x: i32,
    y: i32,
}

#[derive(serde::Serialize)]
struct LiveSpawnObjectCommand<'a> {
    #[serde(rename = "type")]
    command_type: &'static str,
    scene_id: &'a str,
    object_id: u64,
    kind: &'a str,
    x: i32,
    y: i32,
    footprint: LiveFootprint,
}

#[derive(Clone, Copy, serde::Serialize)]
struct LiveFootprint {
    visual_offset_x: i32, visual_offset_y: i32, visual_w: i32, visual_h: i32,
    collision_offset_x: i32, collision_offset_y: i32, collision_w: i32, collision_h: i32,
    interaction_offset_x: i32, interaction_offset_y: i32, interaction_w: i32, interaction_h: i32,
    blocks_movement: bool, occludes_player: bool, fade_when_player_behind: bool,
}

impl From<haven_core::ObjectFootprint> for LiveFootprint {
    fn from(value: haven_core::ObjectFootprint) -> Self {
        Self {
            visual_offset_x: value.visual_offset_x, visual_offset_y: value.visual_offset_y,
            visual_w: value.visual_w, visual_h: value.visual_h,
            collision_offset_x: value.collision_offset_x, collision_offset_y: value.collision_offset_y,
            collision_w: value.collision_w, collision_h: value.collision_h,
            interaction_offset_x: value.interaction_offset_x, interaction_offset_y: value.interaction_offset_y,
            interaction_w: value.interaction_w, interaction_h: value.interaction_h,
            blocks_movement: value.blocks_movement, occludes_player: value.occludes_player,
            fade_when_player_behind: value.fade_when_player_behind,
        }
    }
}

#[derive(serde::Serialize)]
struct LiveDeleteObjectCommand<'a> {
    #[serde(rename = "type")]
    command_type: &'static str,
    scene_id: &'a str,
    object_id: u64,
}

fn publish_live_command<T: serde::Serialize>(
    descriptor: &DevelopmentWorldDescriptor,
    command: T,
) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let sequence = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?.as_micros().min(u64::MAX as u128) as u64;
    let envelope = LiveCommandEnvelope {
        schema: "havenwild.development_live_command.v1",
        world_id: &descriptor.world_id,
        sequence,
        command,
    };
    let directory = repo_root_dir().join("WORKSPACE/development");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join("live_command.json");
    let temporary = directory.join("live_command.json.tmp");
    let bytes = serde_json::to_vec_pretty(&envelope).map_err(|error| error.to_string())?;
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if path.exists() { let _ = std::fs::remove_file(&path); }
    std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

pub(crate) fn publish_live_object_move(
    descriptor: &DevelopmentWorldDescriptor,
    scene_id: &ProjectSceneId,
    object_id: ObjectId,
    x: i32,
    y: i32,
) -> Result<(), String> {
    publish_live_command(descriptor, LiveMoveObjectCommand {
        command_type: "move_object", scene_id: scene_id.as_str(), object_id: object_id.raw(), x, y,
    })
}

pub(crate) fn publish_live_object_spawn(
    descriptor: &DevelopmentWorldDescriptor,
    scene_id: &ProjectSceneId,
    object: haven_core::PlacedObject,
) -> Result<(), String> {
    publish_live_command(descriptor, LiveSpawnObjectCommand {
        command_type: "spawn_object", scene_id: scene_id.as_str(), object_id: object.id.raw(),
        kind: object.kind.code(), x: object.x, y: object.y, footprint: object.footprint.into(),
    })
}

pub(crate) fn publish_live_object_delete(
    descriptor: &DevelopmentWorldDescriptor,
    scene_id: &ProjectSceneId,
    object_id: ObjectId,
) -> Result<(), String> {
    publish_live_command(descriptor, LiveDeleteObjectCommand {
        command_type: "delete_object", scene_id: scene_id.as_str(), object_id: object_id.raw(),
    })
}
