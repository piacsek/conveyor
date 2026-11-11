use crate::app::Column;
use crate::model::builds::Build;

#[derive(Debug, Clone, Default)]
pub struct BuildsStage {
    pub column: Column<Build>,
    pub repo: Option<String>,
}
