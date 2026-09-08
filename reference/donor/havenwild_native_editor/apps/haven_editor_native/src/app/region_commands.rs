use super::*;

impl EditorApp {
    pub(crate) fn add_available_region_node(&mut self) {
        let available = self.model.world.scenes.iter().find(|scene| {
            !self
                .model
                .region_graph
                .nodes
                .iter()
                .any(|node| node.scene_id.as_ref() == Some(&scene.id))
        });
        let Some(scene) = available else {
            self.status_message = "Every loaded scene already has a region node".to_string();
            return;
        };
        let scene_id = scene.id.clone();
        let scene_name = scene.name.clone();
        let scene_biome = scene.biome;
        let index = self.model.region_graph.nodes.len();
        let position = RegionPoint::new(
            0.25 + (index % 4) as f32 * 0.17,
            0.20 + (index / 4) as f32 * 0.18,
        );
        let node_id = RegionNodeId::new(scene_id.code());
        self.model.region_graph.nodes.push(RegionNode {
            id: node_id.clone(),
            label: scene_name.clone(),
            scene_id: Some(scene_id),
            kind: RegionNodeKind::Hub,
            biome: scene_biome,
            position,
        });
        self.selection.set_region_node(node_id);
        self.status_message = format!("Added {} to the region map", scene_name);
    }

    pub(crate) fn remove_selected_region_node(&mut self) {
        let Some(selected_index) = self.selected_region_node_index() else {
            self.status_message = "No region node selected".to_string();
            return;
        };
        let Some(node) = self.model.region_graph.nodes.get(selected_index).cloned() else {
            return;
        };
        self.model
            .region_graph
            .links
            .retain(|link| link.from != node.id && link.to != node.id);
        self.model.region_graph.nodes.remove(selected_index);
        let fallback_index =
            selected_index.min(self.model.region_graph.nodes.len().saturating_sub(1));
        if let Some(selected) = self.model.region_graph.nodes.get(fallback_index) {
            self.selection.set_region_node(selected.id.clone());
        } else {
            self.selection.clear();
        }
        self.status_message = format!("Removed region node {}", node.label);
    }
}
