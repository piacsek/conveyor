use crate::app::App;

use super::Menu;

impl App {
    pub(super) fn builds_menu(&self) -> Result<Menu, String> {
        let build = self.builds.column.selected().ok_or("nothing selected")?;
        Ok(Menu {
            title: format!("Manage run {}", build.run_number),
            offers: vec![self.rerun_offer()],
        })
    }
}
