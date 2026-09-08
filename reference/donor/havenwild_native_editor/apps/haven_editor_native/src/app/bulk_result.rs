use super::*;

impl EditorApp {
    pub(crate) fn finish_bulk_result(
        &mut self,
        result: Result<haven_editor::SceneEditOutcome, String>,
    ) {
        match result {
            Ok(outcome) => self.status_message = outcome.message,
            Err(error) => self.status_message = error,
        }
        self.clamp_editor_selection();
    }
}
