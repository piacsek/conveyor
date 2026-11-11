use crate::app::Column;
use crate::model::prs::PullRequest;

#[derive(Debug, Clone, Default)]
pub struct PrsStage {
    pub column: Column<PullRequest>,
}
