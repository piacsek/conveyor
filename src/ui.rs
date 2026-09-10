use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, ColumnState};

pub fn draw(frame: &mut Frame, app: &App) {
    let columns = Layout::horizontal([Constraint::Fill(1); 4]).split(frame.area());
    let (title, body) = match &app.prs {
        ColumnState::Loading => ("My PRs".to_string(), "fetching…"),
        ColumnState::Ready(prs) if prs.is_empty() => {
            (format!("My PRs ({})", prs.len()), "no open pull requests")
        }
        ColumnState::Ready(prs) => (format!("My PRs ({})", prs.len()), ""),
    };
    frame.render_widget(
        Paragraph::new(body).block(Block::bordered().title(title)),
        columns[0],
    );
    for (area, name) in columns[1..]
        .iter()
        .zip(["Merge queue", "Main builds", "Deployed"])
    {
        frame.render_widget(
            Paragraph::new("not configured").block(Block::bordered().title(name)),
            *area,
        );
    }
}
