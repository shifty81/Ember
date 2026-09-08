use crate::transactions::EditTransaction;
use haven_core::GameWorld;

#[derive(Clone, Copy)]
enum BatchDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransactionBatch {
    pub label: String,
    pub transactions: Vec<EditTransaction>,
}

impl EditTransactionBatch {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            transactions: Vec::new(),
        }
    }

    pub fn push(&mut self, transaction: EditTransaction) {
        if transaction.is_empty() {
            return;
        }
        if let Some(existing) = self
            .transactions
            .iter_mut()
            .find(|existing| existing.scene_id == transaction.scene_id)
        {
            existing.extend(transaction.operations);
            return;
        }
        self.transactions.push(transaction);
    }

    pub fn extend(&mut self, transactions: impl IntoIterator<Item = EditTransaction>) {
        for transaction in transactions {
            self.push(transaction);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn operation_count(&self) -> usize {
        self.transactions
            .iter()
            .map(EditTransaction::operation_count)
            .sum()
    }

    pub fn scene_count(&self) -> usize {
        self.transactions.len()
    }

    pub fn apply(&self, world: &mut GameWorld) -> Result<(), String> {
        self.execute_atomically(world, BatchDirection::Forward)
    }

    pub fn revert(&self, world: &mut GameWorld) -> Result<(), String> {
        self.execute_atomically(world, BatchDirection::Reverse)
    }

    fn execute_atomically(
        &self,
        world: &mut GameWorld,
        direction: BatchDirection,
    ) -> Result<(), String> {
        let backup = world.clone();
        let result = match direction {
            BatchDirection::Forward => self
                .transactions
                .iter()
                .try_for_each(|transaction| transaction.apply(world)),
            BatchDirection::Reverse => self
                .transactions
                .iter()
                .rev()
                .try_for_each(|transaction| transaction.revert(world)),
        };
        if let Err(error) = result {
            *world = backup;
            return Err(error);
        }
        Ok(())
    }
}
