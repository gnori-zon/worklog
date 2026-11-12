use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::view_model::{
    AppModel, Focus, FocusMsg, Msg,
    entries::{EditEntryMsg, EntriesModelMode, EntriesMsg, EntryEditModelMode},
    projects::{ProjectModelMode, ProjectsMsg},
};

pub fn map_input(model: &AppModel, event: Event) -> Option<Msg> {
    match model.focus {
        Focus::Projects => map_projects_input(model, event),
        Focus::Entries => map_entries_input(model, event),
    }
}

fn map_projects_input(model: &AppModel, event: Event) -> Option<Msg> {
    match model.projects.mode {
        ProjectModelMode::View(_) => match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Enter => Some(Msg::Focus(FocusMsg::Entries)),
                    KeyCode::Char('/') => Some(Msg::Projects(ProjectsMsg::Search)),
                    KeyCode::Char('n') => Some(Msg::Projects(ProjectsMsg::Create)),
                    KeyCode::Char('r') if model.projects.cursor.selected.is_some() => {
                        Some(Msg::Projects(ProjectsMsg::Rename))
                    }
                    KeyCode::Char('d')
                        if !key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && model.projects.cursor.selected.is_some() =>
                    {
                        Some(Msg::Projects(ProjectsMsg::Delete))
                    }
                    KeyCode::Char('j') => Some(Msg::Projects(ProjectsMsg::SelectNext)),
                    KeyCode::Char('k') => Some(Msg::Projects(ProjectsMsg::SelectPrev)),
                    _ => map_input_for_view_mode(event),
                }
            }

            _ => map_input_for_view_mode(event),
        },
        ProjectModelMode::Create(_)
        | ProjectModelMode::Rename(_)
        | ProjectModelMode::Search(_)
        | ProjectModelMode::Delete(_) => match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char(value) => Some(Msg::Projects(ProjectsMsg::Input(value))),
                    KeyCode::Backspace => Some(Msg::Projects(ProjectsMsg::Backspace)),
                    KeyCode::Esc => Some(Msg::Projects(ProjectsMsg::Cancel)),
                    KeyCode::Enter => Some(Msg::Projects(ProjectsMsg::Submit)),
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

fn map_entries_input(model: &AppModel, event: Event) -> Option<Msg> {
    match model.entries.mode.clone() {
        EntriesModelMode::View(_) => match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                let current_project = model.projects.get_selected().map(|project| project.id);
                match key_event.code {
                    KeyCode::Esc => Some(Msg::Focus(FocusMsg::Projects)),
                    KeyCode::Char('/') => Some(Msg::Entries(EntriesMsg::Search(current_project))),
                    KeyCode::Char('n') if current_project.is_some() => {
                        Some(Msg::Entries(EntriesMsg::Create(current_project.unwrap())))
                    }
                    KeyCode::Char('r') if model.entries.cursor.selected.is_some() => {
                        Some(Msg::Entries(EntriesMsg::Rename))
                    }
                    KeyCode::Char('i') if model.entries.cursor.selected.is_some() => {
                        Some(Msg::Entries(EntriesMsg::Edit(EditEntryMsg::Start)))
                    }
                    KeyCode::Char('d')
                        if !key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && model.entries.cursor.selected.is_some() =>
                    {
                        Some(Msg::Entries(EntriesMsg::Delete))
                    }
                    KeyCode::Char(' ') if model.entries.cursor.selected.is_some() => {
                        let selected = model.entries.get_selected().unwrap();
                        if selected.ended_at.is_some() {
                            Some(Msg::Entries(EntriesMsg::ClearFinishEntry))
                        } else {
                            Some(Msg::Entries(EntriesMsg::FinishEntry))
                        }
                    }
                    KeyCode::Char('j') => Some(Msg::Entries(EntriesMsg::SelectNext)),
                    KeyCode::Char('k') => Some(Msg::Entries(EntriesMsg::SelectPrev)),
                    KeyCode::Tab => Some(Msg::Entries(EntriesMsg::SelectNextFilter)),
                    KeyCode::BackTab => Some(Msg::Entries(EntriesMsg::SelectPrevFilter)),
                    _ => map_input_for_view_mode(event),
                }
            }

            _ => map_input_for_view_mode(event),
        },
        EntriesModelMode::Edit(_, edit_mode) => match edit_mode {
            EntryEditModelMode::View(_) => match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Char('j') => Some(Msg::Entries(EntriesMsg::Edit(
                            EditEntryMsg::SelectNextField,
                        ))),
                        KeyCode::Char('k') => Some(Msg::Entries(EntriesMsg::Edit(
                            EditEntryMsg::SelectPrevField,
                        ))),
                        KeyCode::Char('i') => {
                            Some(Msg::Entries(EntriesMsg::Edit(EditEntryMsg::EditField)))
                        }
                        KeyCode::Esc => Some(Msg::Entries(EntriesMsg::Cancel)),
                        KeyCode::Enter => Some(Msg::Entries(EntriesMsg::Submit)),
                        _ => None,
                    }
                }
                _ => None,
            },
            EntryEditModelMode::EditStartedAt(_) | EntryEditModelMode::EditEndedAt(_) => {
                match event {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        match key_event.code {
                            KeyCode::Char(value) => Some(Msg::Entries(EntriesMsg::Input(value))),
                            KeyCode::Backspace => Some(Msg::Entries(EntriesMsg::Backspace)),
                            KeyCode::Esc => Some(Msg::Entries(EntriesMsg::Cancel)),
                            KeyCode::Enter => Some(Msg::Entries(EntriesMsg::Submit)),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
        },
        EntriesModelMode::Create(_, _)
        | EntriesModelMode::Rename(_)
        | EntriesModelMode::Search(_, _)
        | EntriesModelMode::Delete(_) => match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char(value) => Some(Msg::Entries(EntriesMsg::Input(value))),
                    KeyCode::Backspace => Some(Msg::Entries(EntriesMsg::Backspace)),
                    KeyCode::Esc => Some(Msg::Entries(EntriesMsg::Cancel)),
                    KeyCode::Enter => Some(Msg::Entries(EntriesMsg::Submit)),
                    _ => None,
                }
            }
            _ => None,
        },
    }
}

fn map_input_for_view_mode(event: Event) -> Option<Msg> {
    match event {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match key_event.code {
            KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Msg::Quit)
            }
            _ => None,
        },
        _ => None,
    }
}
