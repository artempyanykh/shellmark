use anyhow::Result;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

use super::*;
use crate::{keys::ModeMap, storage::friendly_path};
use std::{io::Stderr, iter::FromIterator, u16};

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
    new_state: &BrowseState,
    keybinds: &ModeMap<Command>,
) -> Result<()> {
    let mut cursor_loc = CursorLoc::new(0, 0);

    terminal.hide_cursor()?;
    terminal.draw(|f| {
        let all_area = f.size();
        let block = Block::default().title("Shellmark").borders(Borders::ALL);
        let block_inner = block.inner(all_area);
        f.render_widget(block, all_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
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
            Paragraph::new(Span::raw(String::from_iter(&new_state.input.input)))
                .alignment(Alignment::Left),
            input_block_area,
        );

        let list_area = Layout::default()
            .horizontal_margin(1)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])[0];
        let mut rows = Vec::with_capacity(new_state.selection.candidates.len());
        for &sel_idx in &new_state.selection.candidates {
            assert!(
                sel_idx < new_state.bookmarks.len(),
                "Selection index is out of range: {} ∉ ({}, {})",
                sel_idx,
                0,
                new_state.bookmarks.len()
            );
            // Render bookmark name with some colorization
            let bm_name =
                colorize_match(&new_state.bookmarks[sel_idx].name, &new_state.input.input);
            let bm_name = Cell::from(bm_name).style(Style::default().fg(Color::Green));
            // Render bookmark dest with some colorization
            let bm_dest = colorize_match(
                &friendly_path(&new_state.bookmarks[sel_idx].dest),
                &new_state.input.input,
            );
            let bm_dest = Cell::from(bm_dest);
            let row = Row::new(vec![bm_name, bm_dest]);
            rows.push(row);
        }
        let bookmarks_tbl = Table::new(rows)
            .block(Block::default())
            .column_spacing(1)
            .widths(&[Constraint::Min(20), Constraint::Min(80)])
            .highlight_symbol(">> ")
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        let mut bookmarks_state = TableState::default();
        bookmarks_state.select(new_state.selection.selected);

        f.render_stateful_widget(bookmarks_tbl, list_area, &mut bookmarks_state);

        // Render bottom bar
        let bottom_area = chunks[2];
        let bottom_block = Block::default().borders(Borders::TOP);
        let bottom_block_area = bottom_block.inner(bottom_area);
        f.render_widget(bottom_block, bottom_area);

        let key_style = Style::default().add_modifier(Modifier::BOLD);
        let key_desk_style = Style::default().add_modifier(Modifier::ITALIC);
        let help_text = Spans::from(vec![
            Span::styled("[F1]", key_style),
            Span::raw(" "),
            Span::styled("Help", key_desk_style),
            Span::raw(" "),
            Span::styled("[Enter]", key_style),
            Span::raw(" "),
            Span::styled("DWIM", key_desk_style),
            Span::raw(" "),
            Span::styled("[C-j]", key_style),
            Span::raw(" "),
            Span::styled("Jump", key_desk_style),
            Span::raw(" "),
            Span::styled("[C-o]", key_style),
            Span::raw(" "),
            Span::styled("Edit", key_desk_style),
        ]);

        f.render_widget(
            Paragraph::new(help_text).alignment(Alignment::Left),
            bottom_block_area,
        );

        // Render confirmation dialog for bookmark delete
        if new_state.mode == Mode::PendingDelete {
            render_confirm_delete_dialog(f, block_inner);
        }

        if new_state.mode == Mode::Help {
            render_help_window(f, block_inner, keybinds, Mode::Normal);
        }

        cursor_loc = CursorLoc::new(
            input_block_area.x + new_state.input.cursor,
            input_block_area.y,
        );
    })?;

    terminal.set_cursor(cursor_loc.x, cursor_loc.y)?;
    if new_state.mode == Mode::Normal {
        terminal.show_cursor()?;
    } else {
        terminal.hide_cursor()?;
    }

    Ok(())
}

fn render_confirm_delete_dialog<B: Backend>(f: &mut Frame<B>, outer: Rect) {
    let question_text = Span::styled(
        "Delete selected bookmark?",
        Style::default().add_modifier(Modifier::BOLD),
    );
    let question_text_len = question_text.content.len() as u16 + 10;
    let confirmation_text = Spans::from(vec![
        Span::raw("["),
        Span::styled("Y", Style::default().add_modifier(Modifier::UNDERLINED)),
        Span::raw("]es"),
        Span::raw("  "),
        Span::raw("["),
        Span::styled("N", Style::default().add_modifier(Modifier::UNDERLINED)),
        Span::raw("]o"),
    ]);

    let content = Paragraph::new(vec![
        Span::raw("").into(), // empty line
        question_text.into(),
        Span::raw("").into(), // empty line
        confirmation_text,
        Span::raw("").into(), // empty line
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);

    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Ratio(1, 3),
            Constraint::Length(7),
            Constraint::Ratio(1, 3),
        ])
        .split(outer);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Ratio(1, 3),
            Constraint::Length(question_text_len),
            Constraint::Ratio(1, 3),
        ])
        .split(vchunks[1]);

    let dialog_chunk = hchunks[1];

    f.render_widget(Clear, dialog_chunk);
    f.render_widget(content, dialog_chunk);
}

fn render_help_window<B: Backend>(
    f: &mut Frame<B>,
    outer: Rect,
    keybinds: &ModeMap<Command>,
    mode: Mode,
) {
    f.render_widget(Clear, outer);

    // Prepare basic layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(outer);
    let header_chunk = chunks[0];
    let table_chunk = chunks[2];
    let bottom_chunk = chunks[3];

    // Render the header
    let header_text = Span::styled(
        "Key bindings",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
    );
    let header = Paragraph::new(Spans::from(header_text))
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center);

    f.render_widget(header, header_chunk);

    // Render the key bindings
    let table_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(table_chunk);
    let table_left_chunk = table_chunks[0];
    let table_right_chunk = table_chunks[1];

    let mut table_left_lines = vec![];
    let mut table_right_lines = vec![];
    for (combo_desc, action_desc) in keybinds.descriptions(mode) {
        let combo_line = Spans::from(vec![
            Span::styled(combo_desc, Style::default().add_modifier(Modifier::BOLD)),
            Span::from(": "),
        ]);
        table_left_lines.push(combo_line);

        let action_line = Spans::from(action_desc);
        table_right_lines.push(action_line);
    }

    let table_left = Paragraph::new(table_left_lines).alignment(Alignment::Right);
    let table_right = Paragraph::new(table_right_lines).alignment(Alignment::Left);

    f.render_widget(table_left, table_left_chunk);
    f.render_widget(table_right, table_right_chunk);

    // Render bottom bar
    let bottom_block = Block::default().borders(Borders::TOP);
    let bottom_block_area = bottom_block.inner(bottom_chunk);
    f.render_widget(bottom_block, bottom_chunk);

    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let key_desk_style = Style::default().add_modifier(Modifier::ITALIC);
    let help_text = Spans::from(vec![
        Span::styled("[Esc]", key_style),
        Span::raw(" "),
        Span::styled("Exit", key_desk_style),
    ]);

    f.render_widget(
        Paragraph::new(help_text).alignment(Alignment::Left),
        bottom_block_area,
    );
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
