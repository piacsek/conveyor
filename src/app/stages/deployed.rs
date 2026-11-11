use crate::app::Column;
use crate::model::deployed::Deployment;

#[derive(Debug, Clone, Default)]
pub struct DeployedStage {
    pub column: Column<Deployment>,
    pub system: Option<String>,
}
