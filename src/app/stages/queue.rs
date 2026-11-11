use crate::app::Column;
use crate::model::queue::QueueEntry;

#[derive(Debug, Clone, Default)]
pub struct QueueStage {
    pub column: Column<QueueEntry>,
    pub repo: Option<String>,
}
