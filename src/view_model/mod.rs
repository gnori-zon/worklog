pub mod entries;
pub mod projects;

use crate::{Service, repository};

pub struct AppModel {
    pub should_quit: bool,
    pub version: String,
    pub focus: Focus,
    pub projects: projects::ProjectsModel,
    pub entries: entries::EntriesModel,
}

impl AppModel {
    pub fn handle_msg<S: Service>(&mut self, msg: Msg, service: &S) -> Option<Msg> {
        match msg {
            Msg::Focus(focus_msg) => self.handle_focus_msg(focus_msg),
            Msg::Projects(projects_msg) => self.projects.handle_msg(projects_msg, service),
            Msg::Entries(entries_msg) => self.entries.handle_msg(entries_msg, service),
            Msg::Quit => {
                self.should_quit = true;
                None
            }
        }
    }

    fn handle_focus_msg(&mut self, msg: FocusMsg) -> Option<Msg> {
        match msg {
            FocusMsg::SelectNext => self.focus = self.focus.next(),
            FocusMsg::SelectPrev => self.focus = self.focus.prev(),
        };
        None
    }
}

pub enum Msg {
    Focus(FocusMsg),
    Projects(projects::ProjectsMsg),
    Entries(entries::EntriesMsg),
    Quit,
}

pub enum FocusMsg {
    SelectNext,
    SelectPrev,
}

fn create_message(err: repository::Error) -> String {
    match err {
        repository::Error::NotExistsById(_) => String::from("not found by id"),
        repository::Error::NameAlreadyUsed(_) => String::from("name already used"),
        repository::Error::IdAlreadyUsed(_) => String::from("id already used"),
        repository::Error::BrokenState(broken_state) => {
            let broken_state_message = match broken_state {
                repository::BrokenState::HasDuplicateIds(value) => {
                    format!("has duplicated ids: {}", value)
                }
                repository::BrokenState::HasDuplicateNames(value) => {
                    format!("has duplicated names: {}", value)
                }
            };
            format!("internal_error: {}", broken_state_message)
        }
        repository::Error::Json(error) => format!("internal error: {}", error.to_string()),
        repository::Error::Io(error) => format!("internal error: {}", error.to_string()),
        repository::Error::Uuid(error) => format!("internal error: {}", error.to_string()),
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Focus {
    ProjectsView,
    EntriesView,
    StatsView,
}

impl Focus {
    fn next(&self) -> Focus {
        match self {
            Focus::ProjectsView => Focus::EntriesView,
            Focus::EntriesView => Focus::StatsView,
            Focus::StatsView => self.clone(),
        }
    }

    fn prev(&self) -> Focus {
        match self {
            Focus::ProjectsView => self.clone(),
            Focus::EntriesView => Focus::ProjectsView,
            Focus::StatsView => Focus::EntriesView,
        }
    }
}

#[derive(Default)]
pub struct Cursor {
    pub selected: Option<usize>,
}

impl Cursor {
    fn increment(&mut self, max: usize) {
        if max == 0 {
            self.selected = None;
            return;
        }
        let Some(selected) = self.selected else {
            self.selected = Some(0);
            return;
        };
        if selected == usize::MAX {
            return;
        }
        let new_selected = std::cmp::min(selected + 1, max - 1);
        self.selected = Some(new_selected);
    }

    fn decrement(&mut self, max: usize) {
        if max == 0 {
            self.selected = None;
            return;
        }
        let Some(selected) = self.selected else {
            return;
        };
        if selected == usize::MIN {
            return;
        }
        let new_selected = std::cmp::max(selected - 1, 0);
        self.selected = Some(new_selected);
    }

    fn reset(&mut self, max: usize) {
        self.selected = match self.selected {
            Some(selected) => {
                if max == 0 {
                    None
                } else {
                    Some(std::cmp::min(selected, max - 1))
                }
            }
            None => None,
        }
    }
}

#[derive(Default, Clone)]
pub struct TextInput {
    pub text: String,
}

#[derive(Clone)]
pub struct Notification {
    pub text: String,
    pub level: NotificationLevel,
}

#[derive(Clone)]
pub enum NotificationLevel {
    Success,
    Failure,
}

impl TextInput {
    fn push(&mut self, ch: char) {
        self.text.push(ch);
    }

    fn backspace(&mut self) {
        self.text.pop();
    }
}
