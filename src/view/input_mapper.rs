use crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::view_model::{
    AppModel, Focus, FocusMsg, Msg,
    entries::{EntriesModelMode, EntriesMsg},
    projects::{ProjectModelMode, ProjectsMsg},
};

pub fn map_input(model: &AppModel, event: Event) -> Option<Msg> {
    match model.focus {
        Focus::ProjectsView => match model.projects.mode {
            ProjectModelMode::View(_) => match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Char('/') => Some(Msg::Projects(ProjectsMsg::Search)),
                        KeyCode::Char('n') => Some(Msg::Projects(ProjectsMsg::Create)),
                        KeyCode::Char('r') if model.projects.cursor.selected.is_some() => {
                            Some(Msg::Projects(ProjectsMsg::Rename))
                        }
                        KeyCode::Char('d') if model.projects.cursor.selected.is_some() => {
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
        },
        Focus::EntriesView => match model.entries.mode {
            EntriesModelMode::View(_) => match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    let current_project = model.projects.get_selected().map(|project| project.id);
                    match key_event.code {
                        KeyCode::Char('/') => {
                            Some(Msg::Entries(EntriesMsg::Search(current_project)))
                        }
                        KeyCode::Char('n') if current_project.is_some() => {
                            Some(Msg::Entries(EntriesMsg::Create(current_project.unwrap())))
                        }
                        KeyCode::Char('r') if model.entries.cursor.selected.is_some() => {
                            Some(Msg::Entries(EntriesMsg::Rename))
                        }
                        KeyCode::Char('d') if model.entries.cursor.selected.is_some() => {
                            Some(Msg::Entries(EntriesMsg::Delete))
                        }
                        KeyCode::Char('f') if model.entries.cursor.selected.is_some() => {
                            Some(Msg::Entries(EntriesMsg::FinishEntry))
                        }
                        KeyCode::Char('F') if model.entries.cursor.selected.is_some() => {
                            Some(Msg::Entries(EntriesMsg::ClearFinishEntry))
                        }
                        KeyCode::Char('j') => Some(Msg::Entries(EntriesMsg::SelectNext)),
                        KeyCode::Char('k') => Some(Msg::Entries(EntriesMsg::SelectPrev)),
                        _ => map_input_for_view_mode(event),
                    }
                }

                _ => map_input_for_view_mode(event),
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
        },
        Focus::StatsView => map_input_for_view_mode(event),
    }
}

fn map_input_for_view_mode(event: Event) -> Option<Msg> {
    match event {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match key_event.code {
            KeyCode::Char('q') => Some(Msg::Quit),
            KeyCode::Char('l') => Some(Msg::Focus(FocusMsg::SelectNext)),
            KeyCode::Char('h') => Some(Msg::Focus(FocusMsg::SelectPrev)),
            _ => None,
        },
        _ => None,
    }
}
