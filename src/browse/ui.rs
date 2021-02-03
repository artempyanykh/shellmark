use anyhow::Result;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Terminal,
};

use super::*;
use crate::storage::friendly_path;
use std::{io::Stderr, iter::FromIterator};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CursorLoc {
    x: u16,
    y: u16,
}

impl CursorLoc {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

pub fn draw_ui(
    terminal: &mut Terminal<CrosstermBackend<Stderr>>,
    new_state: &AppState,
) -> Result<()> {
    let mut cursor_loc = CursorLoc::new(0, 0);

    terminal.draw(|f| {
        let all_area = f.size();
        let block = Block::default().title("Shellmark").borders(Borders::ALL);
        let block_inner = block.inner(all_area);
        f.render_widget(block, all_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .margin(0)
            .split(block_inner);

        let input_area = chunks[0];
        let input_block = Block::default().borders(Borders::BOTTOM);
        let input_block_area = input_block.inner(chunks[0]);

        let input_area_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Min(2)])
            .split(input_block_area);
        let input_symbol_area = input_area_chunk[0];
        let input_block_area = input_area_chunk[1];

        f.render_widget(input_block, input_area);
        f.render_widget(
            Paragraph::new(Span::raw(">")).alignment(Alignment::Center),
            input_symbol_area,
        );

        f.render_widget(
            Paragraph::new(Span::raw(String::from_iter(&new_state.input_state.input)))
                .alignment(Alignment::Left),
            input_block_area,
        );

        let list_area = Layout::default()
            .horizontal_margin(1)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];
        let mut rows = Vec::with_capacity(new_state.selection_state.selection.len());
        for &sel_idx in &new_state.selection_state.selection {
            assert!(
                sel_idx < new_state.bookmarks.len(),
                "Selection index is out of range: {} ∉ ({}, {})",
                sel_idx,
                0,
                new_state.bookmarks.len()
            );
            // Render bookmark name with some colorization
            let bm_name = colorize_match(
                &new_state.bookmarks[sel_idx].name,
                &new_state.input_state.input,
            );
            let bm_name = Cell::from(bm_name).style(Style::default().fg(Color::Green));
            // Render bookmark dest with some colorization
            let bm_dest = colorize_match(
                &friendly_path(&new_state.bookmarks[sel_idx].dest),
                &new_state.input_state.input,
            );
            let bm_dest = Cell::from(bm_dest);
            let row = Row::new(vec![bm_name, bm_dest]);
            rows.push(row);
        }
        let bookmarks_tbl = Table::new(rows)
            .block(Block::default())
            .column_spacing(1)
            .widths(&[Constraint::Min(10), Constraint::Min(10)])
            .highlight_symbol(">> ")
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        let mut bookmarks_state = TableState::default();
        bookmarks_state.select(new_state.selection_state.highlight);

        f.render_stateful_widget(bookmarks_tbl, list_area, &mut bookmarks_state);

        cursor_loc = CursorLoc::new(
            input_block_area.x + new_state.input_state.cursor,
            input_block_area.y,
        );
    })?;

    terminal.set_cursor(cursor_loc.x, cursor_loc.y)?;
    terminal.show_cursor()?;

    Ok(())
}

fn colorize_match(str: &str, input: &[char]) -> Spans<'static> {
    let mut spans = Vec::new();
    let mut cur_span: Option<(bool, Vec<char>)> = None;
    let mut match_idx = 0;

    for ch in str.chars() {
        if match_idx < input.len()
            && ch.to_lowercase().to_string() == input[match_idx].to_lowercase().to_string()
        {
            // We have a match
            match &mut cur_span {
                None => cur_span = Some((true, vec![ch])),
                Some(existing_span) => {
                    if existing_span.0 {
                        existing_span.1.push(ch);
                    } else {
                        spans.push(colorize_span(existing_span));
                        cur_span = Some((true, vec![ch]));
                    }
                }
            }

            match_idx += 1;
        } else {
            // No match
            match &mut cur_span {
                None => cur_span = Some((false, vec![ch])),
                Some(existing_span) => {
                    if !existing_span.0 {
                        existing_span.1.push(ch);
                    } else {
                        spans.push(colorize_span(existing_span));
                        cur_span = Some((false, vec![ch]));
                    }
                }
            }
        }
    }

    if let Some(span) = cur_span {
        spans.push(colorize_span(&span));
    }

    Spans::from(spans)
}

fn colorize_span(span: &(bool, Vec<char>)) -> Span<'static> {
    let (is_match, text) = span;
    let str = String::from_iter(text);
    if *is_match {
        Span::styled(str, Style::default().fg(Color::Red))
    } else {
        Span::raw(str)
    }
}
