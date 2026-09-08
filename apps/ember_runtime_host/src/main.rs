use ember_level::load_level_json;
use ember_runtime::{compile_level, BehaviorLibrary};
use ember_runtime_bridge::{write_snapshot, RuntimeStateSnapshot};
use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("Ember Runtime failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut level_path: Option<PathBuf> = None;
    let mut session_dir: Option<PathBuf> = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--level" => {
                index += 1;
                level_path = args.get(index).map(PathBuf::from);
            }
            "--session-dir" => {
                index += 1;
                session_dir = args.get(index).map(PathBuf::from);
            }
            other => return Err(format!("unknown Ember argument: {other}")),
        }
        index += 1;
    }

    let mut snapshot = RuntimeStateSnapshot {
        schema_version: 1,
        runtime: "Ember Runtime".into(),
        ready: true,
        ..RuntimeStateSnapshot::default()
    };

    if let Some(level_path) = level_path {
        let level = load_level_json(&level_path).map_err(|e| e.to_string())?;
        let mut world =
            compile_level(&level, &BehaviorLibrary::new()).map_err(|e| e.to_string())?;
        world.tick_once()?;
        snapshot.active_level = Some(level.scene.id.to_string());
        snapshot.entity_count = world.entities.len();
        snapshot.semantic_cell_count = world.semantic_cells.len();
    }

    if let Some(session_dir) = session_dir {
        write_snapshot(&session_dir, &snapshot)?;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?
    );
    Ok(())
}
