pub fn editor_scope_note() -> &'static str {
    "Shared editor spine for the in-game overlay, native shell, validation, and web tools."
}

pub fn editor_world_scope_note() -> &'static str {
    "EditorWorldModel combines project, scene registry, region graph, rectangles, and animation validation."
}

pub fn editor_workspace_titles() -> [&'static str; 10] {
    [
        "World & Scene Editor",
        "Tile/Autotile/Water Editor",
        "Object & Furniture Editor",
        "Pixel/Animation Editor",
        "Character/NPC Editor",
        "Dialogue/Quest/Event Editor",
        "Recipe/Item/Crafting Editor",
        "Tavern/Staff/Business Editor",
        "Worldgen/Biome Editor",
        "Validation/Build/Packaging",
    ]
}
