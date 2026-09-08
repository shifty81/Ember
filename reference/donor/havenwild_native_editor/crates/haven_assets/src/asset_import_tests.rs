    use super::*;
    #[test] fn provider_detection_prefers_animation_over_raw_sheet() { let registry = AssetImportRegistry::with_defaults(); let source = ImportSource { path: "walk.png".into(), category_hint: Some(AssetCategory::Animation), tile_size: Some([64,64]), profile_id: None }; assert_eq!(registry.best(&source).unwrap().id(), "sprite_animation_sheet"); }
    #[test] fn tsx_attribute_parser_reads_tileset_values() { let xml = r#"<tileset tilewidth="32" tileheight="32" tilecount="64" columns="8"><image source="terrain.png"/></tileset>"#; assert_eq!(root_attr(xml, "tileset", "tilewidth").as_deref(), Some("32")); assert_eq!(first_tag_attr(xml, "image", "source").as_deref(), Some("terrain.png")); }
    #[test] fn tsx_normalization_reads_wang_animation_properties_and_collision() {
        let xml = r#"<tileset tilewidth="32" tileheight="32" tilecount="1" columns="1"><image source="terrain.png"/><tile id="0" class="Grass Edge" probability="0.5"><properties><property name="footstep" value="grass"/></properties><animation><frame tileid="0" duration="120"/></animation><objectgroup><object id="1" x="0" y="16" width="32" height="16"/></objectgroup></tile><wangsets><wangset name="Natural Ground"><wangtile tileid="0" wangid="1,1,1,1,1,1,1,1"/></wangset></wangsets></tileset>"#;
        let wang = parse_wang_assignments(xml);
        let (attrs, body) = indexed_tag_blocks(xml, "tile").into_iter().next().unwrap();
        let tile = parse_tiled_tile_metadata(0, &attrs, &body, wang.get(&0).cloned().unwrap());
        assert_eq!(tile.class_name.as_deref(), Some("Grass Edge"));
        assert_eq!(tile.animation[0].duration_ms, 120);
        assert_eq!(tile.collision_shapes.len(), 1);
        assert_eq!(tile.wang_assignments[0].wang_id.len(), 8);
        assert_eq!(tile.properties.get("footstep"), Some(&serde_json::json!("grass")));
    }
