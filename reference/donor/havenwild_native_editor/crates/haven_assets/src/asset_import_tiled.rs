impl AssetImportProvider for TiledTsxImportProvider {
    fn id(&self) -> &'static str { "tiled_tsx" }
    fn can_import(&self, source: &ImportSource) -> ImportConfidence { if extension(&source.path) == "tsx" { ImportConfidence::Exact } else { ImportConfidence::Unsupported } }
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String> {
        let text = fs::read_to_string(&source.path).map_err(|e| e.to_string())?;
        let tile_count = root_attr(&text, "tileset", "tilecount").and_then(|v| v.parse().ok()).unwrap_or(0);
        let wang_sets = tag_blocks(&text, "wangset").len();
        let animations = tag_blocks(&text, "animation").len();
        let collision_groups = tag_blocks(&text, "objectgroup").len();
        let properties = tag_blocks(&text, "properties").len();
        let mut unresolved = Vec::new();
        if wang_sets == 0 { unresolved.push("TSX contains no Wang-set terrain metadata".to_string()); }
        Ok(ImportPreview {
            provider_id: self.id().to_string(),
            state: if wang_sets > 0 || animations > 0 || collision_groups > 0 { ImportState::RuntimeReady } else { ImportState::PartiallyConfigured },
            detected_sources: vec![DetectedSource { provider_id: self.id().to_string(), confidence: ImportConfidence::Exact, kind: AssetSourceKind::TiledTileset, path: source_path(&source.path) }],
            asset_count: tile_count,
            source_count: tag_blocks(&text, "image").len().max(1),
            categories: [default_category(source)].into_iter().collect(),
            unresolved,
            diagnostics: vec![ImportDiagnostic { severity: "info".to_string(), code: "HWI-TSX-001".to_string(), message: format!("normalized {wang_sets} Wang sets, {animations} animations, {collision_groups} collision groups, and {properties} property groups") }],
        })
    }
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String> {
        let text = fs::read_to_string(&request.source.path).map_err(|e| e.to_string())?;
        let width: u32 = root_attr(&text, "tileset", "tilewidth").and_then(|v| v.parse().ok()).ok_or("TSX missing tilewidth")?;
        let height: u32 = root_attr(&text, "tileset", "tileheight").and_then(|v| v.parse().ok()).ok_or("TSX missing tileheight")?;
        let columns: u32 = root_attr(&text, "tileset", "columns").and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
        let tile_count: u32 = root_attr(&text, "tileset", "tilecount").and_then(|v| v.parse().ok()).unwrap_or(0);
        let image = first_tag_attr(&text, "image", "source").unwrap_or_else(|| "missing.png".to_string());
        let margin = root_attr(&text, "tileset", "margin").and_then(|v| v.parse().ok()).unwrap_or(0);
        let spacing = root_attr(&text, "tileset", "spacing").and_then(|v| v.parse().ok()).unwrap_or(0);
        let source_id = AssetSourceId("tsx_atlas".to_string());
        let wang_index = parse_wang_assignments(&text);
        let tile_blocks = indexed_tag_blocks(&text, "tile");
        let mut tile_metadata = BTreeMap::new();
        for (attrs, body) in tile_blocks {
            if let Some(id) = attr_value(&attrs, "id").and_then(|value| value.parse::<u32>().ok()) {
                tile_metadata.insert(id, parse_tiled_tile_metadata(id, &attrs, &body, wang_index.get(&id).cloned().unwrap_or_default()));
            }
        }
        let mut assets = Vec::new();
        for index in 0..tile_count {
            let column = index % columns;
            let row = index / columns;
            let id = format!("tile_{index}");
            let mut metadata = BTreeMap::new();
            metadata.insert("tiled_tile_id".to_string(), serde_json::json!(index));
            if let Some(tiled) = tile_metadata.get(&index) { metadata.insert("tiled".to_string(), serde_json::to_value(tiled).map_err(|error| error.to_string())?); }
            let category = default_category(&request.source);
            let semantic = tile_metadata.get(&index).and_then(|item| item.class_name.as_deref()).map(|name| format!("{}.{}", category_prefix(&category), normalize_id(name))).unwrap_or_else(|| format!("tiled.{id}"));
            let mut asset = AssetDefinition { id: AssetId(id.clone()), category: category.clone(), semantic_id: semantic, source_id: source_id.clone(), atlas_region: Some(AtlasRegion { x: margin + column * (width + spacing), y: margin + row * (height + spacing), width, height }), variants: vec![], tags: BTreeSet::new(), metadata };
            if let Some(tiled) = tile_metadata.get(&index) {
                if !tiled.animation.is_empty() {
                    let contract = CategoryMetadata::Animation(AnimationMetadata { schema: CATEGORY_METADATA_SCHEMA.to_string(), clips: vec![AnimationClip { id: "default".to_string(), frames: tiled.animation.clone(), looping: true, direction: None }], default_clip: Some("default".to_string()) });
                    asset.metadata.insert("animation_contract".to_string(), serde_json::to_value(contract).map_err(|error| error.to_string())?);
                    asset.tags.insert("animated".to_string());
                }
                if category == AssetCategory::Terrain {
                    if let Some(wang) = tiled.wang_assignments.first() {
                        let peers = wang_to_peers(&wang.wang_id);
                        let contract = CategoryMetadata::Terrain(Box::new(TerrainMetadata { schema: CATEGORY_METADATA_SCHEMA.to_string(), terrain_set: normalize_id(&wang.wang_set), terrain_id: normalize_id(&wang.wang_set), match_mode: TerrainMatchMode::CornersAndSides, peers, movement_cost: 100, collision: !tiled.collision_shapes.is_empty(), footstep_material: None, seasonal_variants: BTreeMap::new(), pcg_tags: BTreeSet::new() }));
                        attach_category_metadata(&mut asset, &contract)?;
                    }
                }
                if !tiled.collision_shapes.is_empty() { asset.metadata.insert("collision_shapes".to_string(), serde_json::to_value(&tiled.collision_shapes).map_err(|error| error.to_string())?); }
                for key in tiled.properties.keys() { asset.tags.insert(format!("property:{}", normalize_id(key))); }
            }
            assets.push(asset);
        }
        let source = AssetSource { id: source_id, kind: AssetSourceKind::TiledTileset, path: image, tile_size: Some([width, height]), margin, spacing };
        let state = if text.contains("<wangset") || text.contains("<animation") || text.contains("<objectgroup") { ImportState::RuntimeReady } else { ImportState::PartiallyConfigured };
        let mut properties = BTreeMap::new();
        properties.insert("wang_set_count".to_string(), serde_json::json!(tag_blocks(&text, "wangset").len()));
        properties.insert("normalizes_properties".to_string(), serde_json::json!(true));
        properties.insert("normalizes_animation".to_string(), serde_json::json!(true));
        properties.insert("normalizes_collision".to_string(), serde_json::json!(true));
        Ok(ImportedAssetPack { manifest: base_manifest(request, vec![source], assets), state: request.approval_state.unwrap_or(state), diagnostics: vec![], generated_profiles: vec![ImportProfile { schema: "havenwild.import_profile.v1".to_string(), id: format!("{}_tsx", request.pack_id.0), provider: self.id().to_string(), tile_size: Some([width, height]), margin, spacing, properties }] })
    }
}

fn category_prefix(category: &AssetCategory) -> &'static str {
    match category { AssetCategory::Terrain => "terrain", AssetCategory::TileObject => "object", AssetCategory::Building => "building", AssetCategory::Character => "character", AssetCategory::Animation => "animation", AssetCategory::Item => "item", AssetCategory::Audio => "audio", AssetCategory::Ui => "ui", _ => "asset" }
}
fn normalize_id(value: &str) -> String { value.trim().to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>().trim_matches('_').to_string() }
fn attr_value(attrs: &str, name: &str) -> Option<String> { let key = format!("{name}=\""); let start = attrs.find(&key)? + key.len(); let end = attrs[start..].find('"')? + start; Some(attrs[start..end].to_string()) }
fn root_attr(text: &str, tag: &str, name: &str) -> Option<String> { let start = text.find(&format!("<{tag}"))?; let end = text[start..].find('>')? + start; attr_value(&text[start..=end], name) }
fn first_tag_attr(text: &str, tag: &str, name: &str) -> Option<String> { root_attr(text, tag, name) }
fn tag_blocks(text: &str, tag: &str) -> Vec<String> { indexed_tag_blocks(text, tag).into_iter().map(|(_, body)| body).collect() }
fn indexed_tag_blocks(text: &str, tag: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut cursor = 0;
    let open = format!("<{tag}");
    let close = format!("</{tag}>");

    while let Some(relative) = text[cursor..].find(&open) {
        let start = cursor + relative;
        let name_end = start + open.len();
        let boundary = text.as_bytes().get(name_end).copied();
        if !matches!(boundary, Some(b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>')) {
            cursor = name_end;
            continue;
        }

        let Some(head_end_rel) = text[start..].find('>') else {
            break;
        };
        let head_end = start + head_end_rel;
        let attrs = text[name_end..head_end].to_string();
        if text.as_bytes().get(head_end.wrapping_sub(1)) == Some(&b'/') {
            result.push((attrs, String::new()));
            cursor = head_end + 1;
            continue;
        }
        let Some(close_rel) = text[head_end + 1..].find(&close) else {
            break;
        };
        let close_start = head_end + 1 + close_rel;
        result.push((attrs, text[head_end + 1..close_start].to_string()));
        cursor = close_start + close.len();
    }
    result
}
fn parse_property_value(attrs: &str) -> serde_json::Value {
    let raw = attr_value(attrs, "value").unwrap_or_default(); match attr_value(attrs, "type").as_deref() { Some("int") => raw.parse::<i64>().map(serde_json::Value::from).unwrap_or_else(|_| serde_json::Value::String(raw)), Some("float") => raw.parse::<f64>().map(serde_json::Value::from).unwrap_or_else(|_| serde_json::Value::String(raw)), Some("bool") => serde_json::Value::Bool(raw == "true" || raw == "1"), _ => serde_json::Value::String(raw) }
}
fn parse_tiled_tile_metadata(tile_id: u32, attrs: &str, body: &str, wang_assignments: Vec<TiledWangAssignment>) -> TiledTileMetadata {
    let class_name = attr_value(attrs, "class").or_else(|| attr_value(attrs, "type"));
    let probability = attr_value(attrs, "probability").and_then(|value| value.parse().ok());
    let mut properties = BTreeMap::new(); for (property_attrs, _) in indexed_tag_blocks(body, "property") { if let Some(name) = attr_value(&property_attrs, "name") { properties.insert(name, parse_property_value(&property_attrs)); } }
    let mut animation = Vec::new(); for (frame_attrs, _) in indexed_tag_blocks(body, "frame") { if let (Some(tile_id), Some(duration_ms)) = (attr_value(&frame_attrs, "tileid").and_then(|value| value.parse().ok()), attr_value(&frame_attrs, "duration").and_then(|value| value.parse().ok())) { animation.push(AnimationFrame { tile_id, duration_ms }); } }
    let mut collision_shapes = Vec::new(); for (object_attrs, object_body) in indexed_tag_blocks(body, "object") { let x = attr_value(&object_attrs, "x").and_then(|v| v.parse().ok()).unwrap_or(0.0); let y = attr_value(&object_attrs, "y").and_then(|v| v.parse().ok()).unwrap_or(0.0); let width = attr_value(&object_attrs, "width").and_then(|v| v.parse().ok()).unwrap_or(0.0); let height = attr_value(&object_attrs, "height").and_then(|v| v.parse().ok()).unwrap_or(0.0); let kind = if object_body.contains("<polygon") { "polygon" } else if object_body.contains("<ellipse") { "ellipse" } else { "rectangle" }; collision_shapes.push(CollisionShape { kind: kind.to_string(), points: vec![], position: [x, y], size: [width, height] }); }
    TiledTileMetadata { tile_id, class_name, probability, properties, animation, collision_shapes, wang_assignments }
}
fn parse_wang_assignments(text: &str) -> BTreeMap<u32, Vec<TiledWangAssignment>> {
    let mut result: BTreeMap<u32, Vec<TiledWangAssignment>> = BTreeMap::new(); for (attrs, body) in indexed_tag_blocks(text, "wangset") { let name = attr_value(&attrs, "name").unwrap_or_else(|| "terrain".to_string()); for (tile_attrs, _) in indexed_tag_blocks(&body, "wangtile") { if let (Some(tile_id), Some(wang_id)) = (attr_value(&tile_attrs, "tileid").and_then(|v| v.parse().ok()), attr_value(&tile_attrs, "wangid")) { let values = wang_id.split(',').filter_map(|value| value.trim().parse::<u32>().ok()).collect(); result.entry(tile_id).or_default().push(TiledWangAssignment { wang_set: name.clone(), wang_id: values }); } } }
    result
}
fn wang_to_peers(values: &[u32]) -> TerrainPeers {
    let peer = |index: usize| values.get(index).copied().filter(|value| *value != 0).map(|value| format!("wang_{value}")); TerrainPeers { north_east: peer(0), east: peer(1), south_east: peer(2), south: peer(3), south_west: peer(4), west: peer(5), north_west: peer(6), north: peer(7) }
}

pub struct AudioFolderImportProvider;
