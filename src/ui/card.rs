use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::text::{pad_right, truncate};
use crate::ui::style::{BAR, INDENT, dim};

pub(crate) fn card(
    selected: bool,
    glyph: (char, Color),
    icon: Option<(char, Color)>,
    title: String,
    right: String,
    rest: Vec<Vec<Span<'static>>>,
    width: usize,
) -> Vec<Line<'static>> {
    let icon_width = if icon.is_some() { 2 } else { 0 };
    let title_width = width.saturating_sub(INDENT + icon_width + right.chars().count() + 1);
    let emphasis = if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let bar = || {
        if selected {
            Span::styled(BAR, Style::default().fg(Color::Cyan))
        } else {
            Span::raw(" ")
        }
    };
    let mut first = vec![
        bar(),
        Span::raw(" "),
        Span::styled(glyph.0.to_string(), Style::default().fg(glyph.1)),
        Span::raw(" "),
    ];
    if let Some((icon, color)) = icon {
        first.push(Span::styled(icon.to_string(), Style::default().fg(color)));
        first.push(Span::raw(" "));
    }
    first.extend([
        Span::styled(pad_right(&title, title_width), emphasis),
        Span::raw(" "),
        Span::styled(right, dim()),
    ]);
    let mut lines = vec![Line::from(first)];
    lines.extend(rest.into_iter().map(|spans| {
        let mut line = vec![bar(), Span::raw(" ".repeat(INDENT - 1))];
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
