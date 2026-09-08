impl EditorApp {
    pub(crate) fn handle_new_pixel_dialog_input(&mut self) -> bool {
        if self.pixel_studio.new_dialog.is_none() {
            return false;
        }
        if is_key_pressed(KeyCode::Escape) {
            self.pixel_studio.new_dialog = None;
            self.status_message = "Cancelled new Pixel Studio asset".to_string();
            return true;
        }
        if is_key_pressed(KeyCode::Tab) {
            let direction = if is_key_down(KeyCode::LeftShift)
                || is_key_down(KeyCode::RightShift)
            {
                -1
            } else {
                1
            };
            if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                dialog.focused = next_new_pixel_field(dialog.focused, direction);
            }
        }
        let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        if alt_down && is_key_pressed(KeyCode::Left) {
            self.pixel_studio.cycle_new_kind(-1);
            return true;
        }
        if alt_down && is_key_pressed(KeyCode::Right) {
            self.pixel_studio.cycle_new_kind(1);
            return true;
        }
        if is_key_pressed(KeyCode::Backspace) {
            if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                edit_new_pixel_field(dialog, None, true);
            }
        }
        while let Some(character) = get_char_pressed() {
            if !character.is_control() {
                if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                    edit_new_pixel_field(dialog, Some(character), false);
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            match self.pixel_studio.create_from_dialog() {
                Ok(message) => self.status_message = message,
                Err(error) => {
                    if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                        dialog.error = Some(error.clone());
                    }
                    self.status_message = format!("New pixel asset failed: {error}");
                }
            }
        }
        true
    }

    pub(crate) fn handle_new_pixel_dialog_click(&mut self, mouse: Vec2) -> bool {
        let rect = new_pixel_modal_rect();
        if cancel_rect(rect).contains(mouse) {
            self.pixel_studio.new_dialog = None;
            self.status_message = "Cancelled new Pixel Studio asset".to_string();
            return true;
        }
        if create_rect(rect).contains(mouse) {
            match self.pixel_studio.create_from_dialog() {
                Ok(message) => self.status_message = message,
                Err(error) => {
                    if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                        dialog.error = Some(error.clone());
                    }
                    self.status_message = format!("New pixel asset failed: {error}");
                }
            }
            return true;
        }
        for (index, kind) in PixelDocumentKind::ALL.iter().copied().enumerate() {
            if kind_rect(rect, index).contains(mouse) {
                if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                    dialog.select_kind(kind);
                }
                return true;
            }
        }
        if let Some(field) = field_at(rect, mouse) {
            if let Some(dialog) = self.pixel_studio.new_dialog.as_mut() {
                dialog.focused = field;
                dialog.error = None;
            }
        }
        true
    }

}


fn next_new_pixel_field(current: NewPixelField, direction: i32) -> NewPixelField {
    const FIELDS: [NewPixelField; 9] = [
        NewPixelField::Name, NewPixelField::Width, NewPixelField::Height,
        NewPixelField::CellWidth, NewPixelField::CellHeight, NewPixelField::Spacing,
        NewPixelField::Padding, NewPixelField::OffsetX, NewPixelField::OffsetY,
    ];
    let index = FIELDS.iter().position(|field| *field == current).unwrap_or(0) as i32;
    FIELDS[(index + direction).rem_euclid(FIELDS.len() as i32) as usize]
}

fn edit_new_pixel_field(dialog: &mut NewPixelDialogState, character: Option<char>, backspace: bool) {
    dialog.error = None;
    if dialog.focused == NewPixelField::Name {
        if backspace { dialog.spec.display_name.pop(); }
        else if let Some(character) = character { if dialog.spec.display_name.chars().count() < 64 { dialog.spec.display_name.push(character); } }
        return;
    }
    let mut text = match dialog.focused {
        NewPixelField::Width => dialog.spec.width.to_string(),
        NewPixelField::Height => dialog.spec.height.to_string(),
        NewPixelField::CellWidth => dialog.spec.cell_width.to_string(),
        NewPixelField::CellHeight => dialog.spec.cell_height.to_string(),
        NewPixelField::Spacing => dialog.spec.spacing.to_string(),
        NewPixelField::Padding => dialog.spec.padding.to_string(),
        NewPixelField::OffsetX => dialog.spec.offset_x.to_string(),
        NewPixelField::OffsetY => dialog.spec.offset_y.to_string(),
        NewPixelField::Name => String::new(),
    };
    if backspace { text.pop(); }
    else if let Some(character) = character {
        let signed = matches!(dialog.focused, NewPixelField::OffsetX | NewPixelField::OffsetY);
        if character.is_ascii_digit() || (signed && character == '-' && text.is_empty()) { text.push(character); }
    }
    match dialog.focused {
        NewPixelField::Width => dialog.spec.width = text.parse().unwrap_or(0),
        NewPixelField::Height => dialog.spec.height = text.parse().unwrap_or(0),
        NewPixelField::CellWidth => dialog.spec.cell_width = text.parse().unwrap_or(0),
        NewPixelField::CellHeight => dialog.spec.cell_height = text.parse().unwrap_or(0),
        NewPixelField::Spacing => dialog.spec.spacing = text.parse().unwrap_or(0),
        NewPixelField::Padding => dialog.spec.padding = text.parse().unwrap_or(0),
        NewPixelField::OffsetX => dialog.spec.offset_x = text.parse().unwrap_or(0),
        NewPixelField::OffsetY => dialog.spec.offset_y = text.parse().unwrap_or(0),
        NewPixelField::Name => {}
    }
}
