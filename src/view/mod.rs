pub mod input_mapper;

use chrono::{Local, NaiveDate, Utc};
use ratatui::{
    Frame,
    buffer::CellWidth,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::{
    models, repository,
    view_model::{
        AppModel, Focus, Notification, NotificationLevel, TextInput, entries::EntriesModelMode,
        projects::ProjectModelMode,
    },
};

pub struct AppLayout {
    header: HeaderView,
    main: MainView,
    footer: FooterView,
}

impl AppLayout {
    pub fn new() -> Self {
        Self::new_with(HeaderView, MainView, FooterView)
    }

    fn new_with(header: HeaderView, main: MainView, footer: FooterView) -> Self {
        Self {
            header,
            main,
            footer,
        }
    }

    pub fn render(&self, frame: &mut Frame, model: &AppModel) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ]);

        let [header, main, footer] = layout.areas(frame.area());

        self.header.render(frame, header, model);
        self.main.render(frame, main, model);
        self.footer.render(frame, footer, model);
    }
}

trait View {
    fn render(&self, frame: &mut Frame, area: Rect, model: &AppModel);
}

struct HeaderView;

impl View for HeaderView {
    fn render(&self, frame: &mut Frame, area: Rect, model: &AppModel) {
        frame.render_widget(Paragraph::new(format!("Worklog v{}", model.version)), area);
    }
}

struct MainView;

impl View for MainView {
    fn render(&self, frame: &mut Frame, area: Rect, model: &AppModel) {
        let area = render_block(frame, area, Block::bordered());

        let [left, center] =
            Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(80)])
                .areas(area);

        let left = render_block(
            frame,
            left,
            mark_border_as_focused(Block::bordered(), Focus::ProjectsView, model.focus),
        );
        let center = render_block(
            frame,
            center,
            mark_border_as_focused(Block::bordered(), Focus::EntriesView, model.focus),
        );

        // left
        let [left_top, left] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(left);

        let search_text = match &model.projects.query.name {
            repository::ProjectNameQuery::Any => String::new(),
            repository::ProjectNameQuery::Contains(text) => text.to_string(),
        };

        let search_status = Paragraph::new(format!("Projects. Search: {}", search_text));
        frame.render_widget(search_status, left_top);

        let items = model
            .projects
            .values
            .iter()
            .map(|project| ListItem::new(project.name.as_str()));

        let list = List::new(items).highlight_style(Style::default().reversed());

        let mut state = ListState::default();
        state.select(model.projects.cursor.selected);

        frame.render_stateful_widget(list, left, &mut state);

        // center
        let [center_top, center] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(center);
        let search_text = match &model.entries.query.name {
            repository::EntryNameQuery::Any => String::new(),
            repository::EntryNameQuery::Contains(text) => text.to_string(),
        };

        let search_status = Paragraph::new(format!("Entries. Search: {}", search_text));
        frame.render_widget(search_status, center_top);

        let [center_left, center_right] =
            Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).areas(center);
        let chanked_entries: Vec<&[models::Entry]> = model
            .entries
            .values
            .chunk_by(|lhs, rhs| {
                lhs.started_at.with_timezone(&Local).date_naive()
                    == rhs.started_at.with_timezone(&Local).date_naive()
            })
            .collect();

        let entry_list_item: Vec<EntryListItem> = chanked_entries
            .iter()
            .flat_map(|chunk| {
                let mut items: Vec<EntryListItem> = Vec::with_capacity(chunk.len() + 1);
                items.push(EntryListItem::Date(
                    chunk
                        .first()
                        .unwrap()
                        .started_at
                        .with_timezone(&Local)
                        .date_naive(),
                ));
                chunk
                    .iter()
                    .for_each(|entry| items.push(EntryListItem::EntryItem(entry.clone())));
                items
            })
            .collect();

        let selectable_indeces: Vec<usize> = entry_list_item
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_selectable())
            .map(|(idx, _)| idx)
            .collect();

        let statuses = entry_list_item.iter().map(|item| match item {
            EntryListItem::Date(_) => ListItem::new(Line::from(vec![Span::raw(" ")])),
            EntryListItem::EntryItem(entry) => {
                let color = if entry.ended_at.is_none() {
                    Color::Green
                } else {
                    Color::Blue
                };
                ListItem::new(Line::from(vec![Span::styled(
                    "■ ",
                    Style::default().fg(color),
                )]))
            }
        });
        let list_statues = List::new(statuses);
        frame.render_widget(list_statues, center_left);

        let items = entry_list_item.iter().map(|item| match item {
            EntryListItem::Date(naive_date) => ListItem::new(
                Line::from(vec![Span::styled(
                    format!("{}", naive_date.format(DATE_FORMAT)),
                    Style::default(),
                )])
                .centered(),
            ),
            EntryListItem::EntryItem(entry) => ListItem::new(Line::from(vec![
                Span::raw(format!(
                    "{} ",
                    entry.started_at.with_timezone(&Local).format(TIME_FORMAT)
                )),
                Span::raw(format!("{} ", entry.name.as_str())),
                Span::raw(format!(
                    "{}",
                    format_duration(entry.started_at, entry.ended_at)
                )),
            ])),
        });
        let list_items = List::new(items).highlight_style(Style::default().reversed());
        let mut list_items_state = ListState::default();

        let selected_index: Option<usize> = model
            .entries
            .cursor
            .selected
            .and_then(|idx| selectable_indeces.get(idx).map(|real_idx| real_idx.clone()));

        list_items_state.select(selected_index);

        frame.render_stateful_widget(list_items, center_right, &mut list_items_state);
    }
}

enum EntryListItem {
    Date(NaiveDate),
    EntryItem(models::Entry),
}

impl EntryListItem {
    fn is_selectable(&self) -> bool {
        match self {
            EntryListItem::Date(_) => false,
            EntryListItem::EntryItem(_) => true,
        }
    }
}

const TIME_FORMAT: &str = "%H:%M";
const DATE_FORMAT: &str = "%d.%m.%Y";

fn format_duration(start: models::DateTime, end: Option<models::DateTime>) -> String {
    let duration = match end {
        Some(end) => end - start,
        None => Utc::now() - start,
    };

    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn mark_border_as_focused(block: Block, expected: Focus, actual: Focus) -> Block {
    if expected == actual {
        block.border_style(Style::new().yellow())
    } else {
        block
    }
}

fn render_block(frame: &mut Frame, area: Rect, block: Block) -> Rect {
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

struct FooterView;

impl FooterView {
    fn render_help_msg(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new("[q] Quit [r] Rename [n] Create [d] Delete [/] Search"),
            area,
        );
    }

    fn draw_input_field(&self, frame: &mut Frame<'_>, area: Rect, text_input: &TextInput) {
        let paragraph = Paragraph::new(text_input.text.as_str());
        frame.render_widget(paragraph, area);
        frame.set_cursor_position((area.x + text_input.text.cell_width(), area.y));
    }

    fn render_notification(&self, frame: &mut Frame, area: Rect, notification: &Notification) {
        let color = match notification.level {
            NotificationLevel::Success => Color::Green,
            NotificationLevel::Failure => Color::Red,
        };
        let paragraph = Paragraph::new(
            Line::from(notification.text.as_str()).style(Style::default().fg(color)),
        );
        frame.render_widget(paragraph, area);
    }
}

impl View for FooterView {
    fn render(&self, frame: &mut Frame, area: Rect, model: &AppModel) {
        let [notification_or_input_area, help_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        self.render_help_msg(frame, help_area);
        match model.focus {
            Focus::ProjectsView => match &model.projects.mode {
                ProjectModelMode::View(notification) => match notification {
                    Some(notification) => {
                        self.render_notification(frame, notification_or_input_area, notification)
                    }
                    None => {}
                },
                ProjectModelMode::Rename(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                ProjectModelMode::Create(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                ProjectModelMode::Search(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                ProjectModelMode::Delete(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
            },
            Focus::EntriesView => match &model.entries.mode {
                EntriesModelMode::View(notification) => match notification {
                    Some(notification) => {
                        self.render_notification(frame, notification_or_input_area, notification)
                    }
                    None => {}
                },
                EntriesModelMode::Rename(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                EntriesModelMode::Create(_, text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                EntriesModelMode::Search(_, text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
                EntriesModelMode::Delete(text_input) => {
                    self.draw_input_field(frame, notification_or_input_area, text_input)
                }
            },
            Focus::StatsView => {}
        };
    }
}
