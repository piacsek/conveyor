use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::text::{pad_right, truncate};
use crate::ui::style::{BAR, INDENT, dim};

pub(crate) fn card(
    selected: bool,
    glyph: (char, Color),
    title: String,
    right: String,
    rest: Vec<Vec<Span<'static>>>,
    width: usize,
) -> Vec<Line<'static>> {
    let title_width = width.saturating_sub(INDENT + right.chars().count() + 1);
    let (bar, emphasis) = if selected {
        (
            Span::styled(BAR, Style::default().fg(Color::Cyan)),
            Style::default().add_modifier(Modifier::BOLD),
        )
    } else {
        (Span::raw(" "), Style::default())
    };
    let mut lines = vec![Line::from(vec![
        bar,
        Span::raw(" "),
        Span::styled(glyph.0.to_string(), Style::default().fg(glyph.1)),
        Span::raw(" "),
        Span::styled(pad_right(&title, title_width), emphasis),
        Span::raw(" "),
        Span::styled(right, dim()),
    ])];
    lines.extend(rest.into_iter().map(|spans| {
        let mut line = vec![Span::raw(" ".repeat(INDENT))];
        line.extend(spans);
        Line::from(line)
    }));
    lines
}

pub(crate) fn meta(parts: &[String], width: usize) -> Vec<Span<'static>> {
    let text: Vec<&str> = parts
        .iter()
        .map(String::as_str)
        .filter(|part| !part.is_empty())
        .collect();
    vec![Span::styled(
        truncate(&text.join(" · "), width.saturating_sub(INDENT)),
        dim(),
    )]
}
