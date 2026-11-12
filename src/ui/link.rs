use std::num::NonZeroU16;

use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::{Position, Rect};

use crate::open::web_url;

fn linkable(url: &str) -> bool {
    web_url(url).is_ok() && url.bytes().all(|byte| byte.is_ascii_graphic())
}

fn find(buffer: &Buffer, area: Rect, label: &str) -> Option<Position> {
    let width = label.chars().count() as u16;
    area.positions().find(|at| {
        at.x + width <= area.right()
            && label
                .chars()
                .zip(at.x..)
                .all(|(ch, x)| buffer[(x, at.y)].symbol().chars().eq([ch]))
    })
}

pub(crate) fn link(buffer: &mut Buffer, area: Rect, number: u64, url: &str) {
    if !linkable(url) {
        return;
    }
    let label = format!("#{number}");
    let Some(at) = find(buffer, area.intersection(buffer.area), &label) else {
        return;
    };
    let id = format!("{}-{}", at.x, at.y);
    for (ch, x) in label.chars().zip(at.x..) {
        buffer[(x, at.y)]
            .set_symbol(&format!("\x1b]8;id={id};{url}\x1b\\{ch}\x1b]8;;\x1b\\"))
            .set_diff_option(CellDiffOption::ForcedWidth(NonZeroU16::MIN));
    }
}
