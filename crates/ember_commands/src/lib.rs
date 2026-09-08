use ember_core::StableId;

pub trait Command<C>: Send {
    fn label(&self) -> &str;
    fn execute(&mut self, context: &mut C) -> Result<(), CommandError>;
    fn undo(&mut self, context: &mut C) -> Result<(), CommandError>;
}

pub struct CommandHistory<C> {
    undo: Vec<Box<dyn Command<C>>>,
    redo: Vec<Box<dyn Command<C>>>,
}

impl<C> Default for CommandHistory<C> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl<C> CommandHistory<C> {
    pub fn execute(
        &mut self,
        mut command: Box<dyn Command<C>>,
        ctx: &mut C,
    ) -> Result<(), CommandError> {
        command.execute(ctx)?;
        self.undo.push(command);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self, ctx: &mut C) -> Result<bool, CommandError> {
        let Some(mut command) = self.undo.pop() else {
            return Ok(false);
        };
        command.undo(ctx)?;
        self.redo.push(command);
        Ok(true)
    }

    pub fn redo(&mut self, ctx: &mut C) -> Result<bool, CommandError> {
        let Some(mut command) = self.redo.pop() else {
            return Ok(false);
        };
        command.execute(ctx)?;
        self.undo.push(command);
        Ok(true)
    }
}

#[derive(Clone, Debug)]
pub struct TransactionId(pub StableId);

#[derive(Debug)]
pub struct CommandError(pub String);

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests {
    use super::*;

    struct ContextWithoutDefault;

    #[test]
    fn command_history_default_does_not_require_context_default() {
        let _: CommandHistory<ContextWithoutDefault> = CommandHistory::default();
    }
}
