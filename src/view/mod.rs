pub mod input_mapper;

use chrono::{Local, TimeDelta, Utc};
use ratatui::{
    Frame,
    buffer::CellWidth,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table, TableState},
};

use crate::{
    models,
    repository::{self, EntryDateQuery},
    view_model::{
        AppModel, DateInput, Focus, Notification, NotificationLevel,
        entries::{EntriesModelMode, EntryEditField, EntryEditModelMode},
        projects::ProjectModelMode,
    },
};

pub fn render_app(frame: &mut Frame, model: &AppModel) {
    match model.focus {
        Focus::Projects => ProjectsScreen.render(frame, model),
        Focus::Entries => EntriesScreen.render(frame, model),
    }
}

trait Screen {
    fn render(&self, frame: &mut Frame, model: &AppModel);
}

struct ProjectsScreen;
impl Screen for ProjectsScreen {
    fn render(&self, frame: &mut Frame, model: &AppModel) {
        let layout = Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![Span::raw("Worklog")]))
            .title(Line::from(format!("Version: {}", model.version)).right_aligned());
        let main = layout.inner(frame.area());
        frame.render_widget(layout, frame.area());

        let [main, footer, input] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(main);

        // main
        let search_text = match &model.projects.query.name {
            repository::ProjectNameQuery::Any => String::new(),
            repository::ProjectNameQuery::Contains(text) => text.to_string(),
        };
        let header = Row::new(vec![
            Cell::from(format!("Projects: {}", search_text)),
            Cell::from("entries"),
            Cell::from("time (7d)"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));
        let rows: Vec<Row> = model
            .projects
            .values
            .iter()
            .map(|project| {
                let stats = model.projects.stats.get(&project.id).unwrap();
                Row::new(vec![
                    Cell::from(project.name.as_str()),
                    Cell::from(format!("{}", stats.entries_count)),
                    Cell::from(format_duration(stats.week_time)),
                ])
            })
            .collect();

        let width = [
            Constraint::Fill(1),    // name
            Constraint::Length(8),  // entries
            Constraint::Length(15), // time(7d)
        ];
        let table = Table::new(rows, width)
            .header(header)
            .block(Block::default())
            .row_highlight_style(Style::default().reversed())
            .column_spacing(2);
        let mut state = TableState::default();
        state.select(model.projects.cursor.selected);
        frame.render_stateful_widget(table, main, &mut state);

        // input
        match &model.projects.mode {
            ProjectModelMode::View(notification) => match notification {
                Some(notification) => render_notification(frame, input, notification),
                None => {}
            },
            ProjectModelMode::Rename(text_input)
            | ProjectModelMode::Create(text_input)
            | ProjectModelMode::Search(text_input)
            | ProjectModelMode::Delete(text_input) => {
                render_input(frame, input, text_input.text.as_str())
            }
        }

        let mode = match model.projects.mode {
            ProjectModelMode::View(_) => Line::from("--NORMAL--"),
            _ => Line::from("--INSERT--").style(Color::Green),
        };

        // footer
        let hint_bar = Block::default()
            .borders(Borders::TOP)
            .title(mode.bold().alignment(Alignment::Left))
            .title(Line::from(
                [
                    hint("n", "new"),
                    hint("r", "rename"),
                    hint("d", "delete"),
                    hint("enter", "open entries"),
                    hint("/", "search"),
                    hint("<C-d>", "quit"),
                ]
                .concat(),
            ))
            .title_alignment(Alignment::Center);
        frame.render_widget(hint_bar, footer);
    }
}

struct EntriesScreen;
impl Screen for EntriesScreen {
    fn render(&self, frame: &mut Frame, model: &AppModel) {
        let layout = Block::default()
            .borders(Borders::ALL)
            .title(Line::from(vec![
                Span::raw("Worklog"),
                Span::raw(" ▸ "),
                Span::raw(model.projects.get_selected().unwrap().clone().name),
            ]))
            .title(Line::from(hint("Esc", "back")).right_aligned());
        let main = layout.inner(frame.area());
        frame.render_widget(layout, frame.area());

        let [header, divider, main, footer, input] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(main);

        // header
        let [search_status, filter_status] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(header);
        let search_text = match &model.entries.query.name {
            repository::EntryNameQuery::Any => String::new(),
            repository::EntryNameQuery::Contains(text) => text.to_string(),
        };
        frame.render_widget(
            Line::from(vec![Span::raw(format!("Entries: {}", search_text))]),
            search_status,
        );

        let selected_filter = model.entries.query.date.clone();
        let filters: Vec<Span> = EntryDateQuery::all_cases()
            .iter()
            .cloned()
            .enumerate()
            .flat_map(|(i, date_query)| {
                let mut result = vec![];
                if i > 0 {
                    result.push(Span::raw(" "));
                } else {
                    result.push(Span::raw("Filter: "));
                }
                let name = match date_query {
                    EntryDateQuery::Any => "all",
                    EntryDateQuery::Running => "running",
                    EntryDateQuery::Today => "today",
                    EntryDateQuery::Week => "week",
                    EntryDateQuery::Mounth => "mounth",
                };
                let mut filter_span = Span::from(format!(" {} ", name));
                if selected_filter == date_query {
                    filter_span = filter_span.style(Style::default().reversed())
                }
                result.push(Span::raw("[ "));
                result.push(filter_span);
                result.push(Span::raw(" ]"));
                result
            })
            .collect();
        frame.render_widget(Line::from(filters), filter_status);

        // devider
        frame.render_widget(Block::default().borders(Borders::TOP), divider);

        // main
        let rows: Vec<Row> = model
            .entries
            .values
            .iter()
            .map(|entry| {
                let (icon, icon_style, status) = if entry.ended_at.is_some() {
                    ("✓", Style::default().fg(Color::DarkGray), "done")
                } else {
                    (
                        "●",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        "running",
                    )
                };
                Row::new(vec![
                    Cell::from(icon).style(icon_style),
                    Cell::from(status),
                    Cell::from(entry.name.as_str()),
                    Cell::from(format_date_range(entry.started_at, entry.ended_at)),
                    Cell::from(format_duration(duration_from(
                        entry.started_at,
                        entry.ended_at,
                    ))),
                ])
            })
            .collect();

        let width = [
            Constraint::Length(2),  // icon
            Constraint::Length(9),  // status
            Constraint::Fill(1),    // name
            Constraint::Length(36), // date range
            Constraint::Length(15),
        ];
        let table = Table::new(rows, width)
            .block(Block::default())
            .row_highlight_style(Style::default().reversed())
            .column_spacing(2);
        let mut state = TableState::default();
        state.select(model.entries.cursor.selected);
        frame.render_stateful_widget(table, main, &mut state);

        // popup
        if let EntriesModelMode::Edit(entry, edit_mode) = &model.entries.mode {
            let popup_area = centered_rect(60, 30, main);
            frame.render_widget(Clear, popup_area);
            let rows = vec![
                Row::new(vec![
                    Cell::from(String::from("Entry: ")),
                    Cell::from(entry.name.as_str()),
                ]),
                Row::new(vec![
                    Cell::from(String::from("Started at: ")),
                    Cell::from(
                        entry
                            .started_at
                            .with_timezone(&Local)
                            .format(DATE_TIME_FORMAT)
                            .to_string(),
                    ),
                ]),
                Row::new(vec![
                    Cell::from(String::from("Ended at: ")),
                    Cell::from(
                        entry
                            .ended_at
                            .map(|ended_at| {
                                ended_at
                                    .with_timezone(&Local)
                                    .format(DATE_TIME_FORMAT)
                                    .to_string()
                            })
                            .unwrap_or_else(|| String::from("-")),
                    ),
                ]),
                Row::new(vec![
                    Cell::from(String::from("Duration: ")),
                    Cell::from(format_duration(duration_from(
                        entry.started_at,
                        entry.ended_at,
                    ))),
                ]),
            ];

            let width = [
                Constraint::Length(12), // field name
                Constraint::Fill(1),    // value
            ];

            let table = Table::new(rows, width)
                .block(Block::bordered().title(Line::from("").bold().alignment(Alignment::Left)))
                .row_highlight_style(Style::default().reversed())
                .column_spacing(2);
            let mut state = TableState::default();
            match edit_mode {
                EntryEditModelMode::View(model_view) => match model_view.selected_field {
                    EntryEditField::StartedAt => state.select(Some(1)),
                    EntryEditField::EndedAt => state.select(Some(2)),
                },
                EntryEditModelMode::EditStartedAt(_) => state.select(Some(1)),
                EntryEditModelMode::EditEndedAt(_) => state.select(Some(2)),
            }
            frame.render_stateful_widget(table, popup_area, &mut state);
        }

        // input
        match &model.entries.mode {
            EntriesModelMode::View(notification) => match notification {
                Some(notification) => render_notification(frame, input, notification),
                None => {}
            },
            EntriesModelMode::Rename(text_input)
            | EntriesModelMode::Create(_, text_input)
            | EntriesModelMode::Search(_, text_input)
            | EntriesModelMode::Delete(text_input) => {
                render_input(frame, input, text_input.text.as_str())
            }
            EntriesModelMode::Edit(_, edit_mode) => match edit_mode {
                EntryEditModelMode::View(model_view) => match &model_view.notification {
                    Some(notification) => render_notification(frame, input, notification),
                    None => {}
                },
                EntryEditModelMode::EditStartedAt(date_input) => {
                    render_date_input(frame, input, date_input)
                }
                EntryEditModelMode::EditEndedAt(date_input) => {
                    render_date_input(frame, input, date_input)
                }
            },
        }
        let mode = match &model.entries.mode {
            EntriesModelMode::View(_) => Line::from("--NORMAL--"),
            EntriesModelMode::Edit(_, edit_mode) => match edit_mode {
                EntryEditModelMode::View(_) => Line::from("--NORMAL--"),
                _ => Line::from("--INSERT--").style(Color::Green),
            },
            _ => Line::from("--INSERT--").style(Color::Green),
        };

        // footer
        let hint_bar = match &model.entries.mode {
            EntriesModelMode::Edit(_, _) => Block::default()
                .borders(Borders::TOP)
                .title(mode.bold().alignment(Alignment::Left))
                .title(
                    Line::from(
                        [
                            hint("esc", "cancel"),
                            hint("enter", "complete"),
                            hint("i", "edit"),
                        ]
                        .concat(),
                    )
                    .alignment(Alignment::Center),
                ),
            _ => Block::default()
                .borders(Borders::TOP)
                .title(mode.bold().alignment(Alignment::Left))
                .title(
                    Line::from(
                        [
                            hint("n", "new"),
                            hint("r", "rename"),
                            hint("i", "edit"),
                            hint("space", "toggle running"),
                            hint("d", "delete"),
                            hint("/", "search"),
                            hint("<C-d>", "quit"),
                        ]
                        .concat(),
                    )
                    .alignment(Alignment::Center),
                ),
        };
        frame.render_widget(hint_bar, footer);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .split(area);

    Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .split(horizontal[1])[1]
}

fn render_input(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(Line::from(text), area);
    frame.set_cursor_position((area.x + text.cell_width(), area.y));
}

fn render_date_input(frame: &mut Frame, area: Rect, date_input: &DateInput) {
    let date_str = date_input
        .date
        .with_timezone(&Local)
        .format(DATE_TIME_FORMAT)
        .to_string();
    // DATE_TIME_FORMAT = "%Y.%m.%d %H:%M"
    let cursor_idx = match date_input.cursor {
        crate::view_model::DateInputCursor::YearThousand => 0,
        crate::view_model::DateInputCursor::YearHundred => 1,
        crate::view_model::DateInputCursor::YearDecimal => 2,
        crate::view_model::DateInputCursor::YearUnit => 3,
        crate::view_model::DateInputCursor::MonthDecimal => 5, // skip '.'
        crate::view_model::DateInputCursor::MonthUnit => 6,
        crate::view_model::DateInputCursor::DayDecimal => 8, // skip '.'
        crate::view_model::DateInputCursor::DayUnit => 9,
        crate::view_model::DateInputCursor::HourDecimal => 11, // skip ' '
        crate::view_model::DateInputCursor::HourUnit => 12,
        crate::view_model::DateInputCursor::MinuteDecimal => 14, // skip ':'
        crate::view_model::DateInputCursor::MuniteUnit => 15,
    };
    frame.render_widget(Line::from(date_str), area);
    frame.set_cursor_position((area.x + cursor_idx, area.y));
}

fn render_notification(frame: &mut Frame, area: Rect, notification: &Notification) {
    let color = match notification.level {
        NotificationLevel::Success => Color::Green,
        NotificationLevel::Failure => Color::Red,
    };
    frame.render_widget(
        Line::styled(notification.text.as_str(), Style::default().fg(color)),
        area,
    );
}

fn hint(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![hint_key(key), Span::raw(format!(" {} ", label))]
}

fn hint_key(key: &str) -> Span<'static> {
    Span::styled(
        format!("[{}]", key),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

const TIME_FORMAT: &str = "%H:%M";
const DATE_TIME_FORMAT: &str = "%Y.%m.%d %H:%M";

fn format_date_range(start: models::DateTime, end: Option<models::DateTime>) -> String {
    let mut formatted = String::new();
    let local_start = start.with_timezone(&Local);
    formatted.push_str(format!("{} → ", local_start.format(DATE_TIME_FORMAT)).as_str());
    match end {
        Some(end) => {
            let local_end = end.with_timezone(&Local);
            let formatted_end = if end.date_naive().eq(&start.date_naive()) {
                local_end.format(TIME_FORMAT)
            } else {
                local_end.format(DATE_TIME_FORMAT)
            };
            formatted.push_str(format!("{}", formatted_end).as_str());
        }
        None => {
            formatted.push_str("...");
        }
    };
    formatted
}

fn format_duration(duration: TimeDelta) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let mut formatted = String::new();
    if days > 0 {
        formatted.push_str(format!("{days}d ").as_str());
    }
    if hours > 0 {
        formatted.push_str(format!("{hours}h ").as_str());
    }

    formatted.push_str(format!("{minutes}m").as_str());
    formatted
}

fn duration_from(start: models::DateTime, end: Option<models::DateTime>) -> TimeDelta {
    match end {
        Some(end) => end - start,
        None => Utc::now() - start,
    }
}
