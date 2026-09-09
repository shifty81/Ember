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
        context: &mut C,
    ) -> Result<(), CommandError> {
        command.execute(context)?;
        self.undo.push(command);
        self.redo.clear();
        Ok(())
    }

    pub fn execute_transaction(
        &mut self,
        label: impl Into<String>,
        commands: Vec<Box<dyn Command<C>>>,
        context: &mut C,
    ) -> Result<(), CommandError>
    where
        C: 'static,
    {
        self.execute(Box::new(CommandTransaction::new(label, commands)), context)
    }

    pub fn undo(&mut self, context: &mut C) -> Result<bool, CommandError> {
        let Some(mut command) = self.undo.pop() else {
            return Ok(false);
        };
        command.undo(context)?;
        self.redo.push(command);
        Ok(true)
    }

    pub fn redo(&mut self, context: &mut C) -> Result<bool, CommandError> {
        let Some(mut command) = self.redo.pop() else {
            return Ok(false);
        };
        command.execute(context)?;
        self.undo.push(command);
        Ok(true)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

pub struct CommandTransaction<C> {
    label: String,
    commands: Vec<Box<dyn Command<C>>>,
    executed: usize,
}

impl<C> CommandTransaction<C> {
    pub fn new(label: impl Into<String>, commands: Vec<Box<dyn Command<C>>>) -> Self {
        Self {
            label: label.into(),
            commands,
            executed: 0,
        }
    }
}

impl<C> Command<C> for CommandTransaction<C> {
    fn label(&self) -> &str {
        &self.label
    }

    fn execute(&mut self, context: &mut C) -> Result<(), CommandError> {
        self.executed = 0;
        for index in 0..self.commands.len() {
            if let Err(error) = self.commands[index].execute(context) {
                for rollback_index in (0..self.executed).rev() {
                    let _ = self.commands[rollback_index].undo(context);
                }
                self.executed = 0;
                return Err(error);
            }
            self.executed += 1;
        }
        Ok(())
    }

    fn undo(&mut self, context: &mut C) -> Result<(), CommandError> {
        for index in (0..self.executed).rev() {
            self.commands[index].undo(context)?;
        }
        self.executed = 0;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TransactionId(pub StableId);

#[derive(Debug)]
pub struct CommandError(pub String);
impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
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

    #[test]
    fn empty_transaction_compiles_and_executes_for_static_context() {
        let mut history: CommandHistory<ContextWithoutDefault> = CommandHistory::default();
        let mut context = ContextWithoutDefault;
        history
            .execute_transaction("Empty transaction", Vec::new(), &mut context)
            .expect("empty transaction should execute");
        assert!(history.can_undo());
    }
}
