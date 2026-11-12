pub mod entries;
pub mod projects;

use chrono::{Datelike, Local, Timelike};

use crate::{
    Service,
    models::{self},
    repository,
    view_model::DateInputCursor::{
        DayDecimal, DayUnit, HourDecimal, HourUnit, MinuteDecimal, MonthDecimal, MonthUnit,
        MuniteUnit, YearDecimal, YearHundred, YearThousand, YearUnit,
    },
};

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
            Msg::Focus(focus_msg) => self.handle_focus_msg(focus_msg, service),
            Msg::Projects(projects_msg) => self.projects.handle_msg(projects_msg, service),
            Msg::Entries(entries_msg) => self.entries.handle_msg(entries_msg, service),
            Msg::Quit => {
                self.should_quit = true;
                None
            }
        }
    }

    fn handle_focus_msg<S: Service>(&mut self, msg: FocusMsg, service: &S) -> Option<Msg> {
        match msg {
            FocusMsg::Projects => {
                self.focus = Focus::Projects;
                self.projects.reload_values(service);
            }
            FocusMsg::Entries => self.focus = Focus::Entries,
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
    Projects,
    Entries,
}

fn create_message(err: &repository::Error) -> String {
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
    Projects,
    Entries,
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

#[derive(Default, Clone)]
pub struct DateInput {
    pub date: models::DateTime,
    pub cursor: DateInputCursor,
}

#[derive(Clone)]
pub enum DateInputCursor {
    YearThousand,
    YearHundred,
    YearDecimal,
    YearUnit,
    MonthDecimal,
    MonthUnit,
    DayDecimal,
    DayUnit,
    HourDecimal,
    HourUnit,
    MinuteDecimal,
    MuniteUnit,
}

impl Default for DateInputCursor {
    fn default() -> Self {
        DateInputCursor::MuniteUnit
    }
}

impl DateInputCursor {
    fn next(&self) -> Self {
        match self {
            DateInputCursor::YearThousand => YearHundred,
            DateInputCursor::YearHundred => YearDecimal,
            DateInputCursor::YearDecimal => YearUnit,
            DateInputCursor::YearUnit => MonthDecimal,
            DateInputCursor::MonthDecimal => MonthUnit,
            DateInputCursor::MonthUnit => DayDecimal,
            DateInputCursor::DayDecimal => DayUnit,
            DateInputCursor::DayUnit => HourDecimal,
            DateInputCursor::HourDecimal => HourUnit,
            DateInputCursor::HourUnit => MinuteDecimal,
            DateInputCursor::MinuteDecimal => MuniteUnit,
            DateInputCursor::MuniteUnit => MuniteUnit,
        }
    }

    fn prev(&self) -> Self {
        match self {
            DateInputCursor::YearThousand => YearThousand,
            DateInputCursor::YearHundred => YearThousand,
            DateInputCursor::YearDecimal => YearHundred,
            DateInputCursor::YearUnit => YearDecimal,
            DateInputCursor::MonthDecimal => YearUnit,
            DateInputCursor::MonthUnit => MonthDecimal,
            DateInputCursor::DayDecimal => MonthUnit,
            DateInputCursor::DayUnit => DayDecimal,
            DateInputCursor::HourDecimal => DayUnit,
            DateInputCursor::HourUnit => HourDecimal,
            DateInputCursor::MinuteDecimal => HourUnit,
            DateInputCursor::MuniteUnit => MinuteDecimal,
        }
    }
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

impl DateInput {
    fn push_local_tz(&mut self, ch: char) {
        let Some(digit) = ch.to_digit(10).map(|it| it as i32) else {
            return;
        };
        let mut local = self.date.with_timezone(&Local);
        let changed = match self.cursor {
            DateInputCursor::YearThousand => {
                let new_year = local.year() % 1000 + digit * 1000;
                if let Some(changed) = local.with_year(new_year) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::YearHundred => {
                let new_year = local.year() / 1000 * 1000 + local.year() % 100 + digit * 100;
                if let Some(changed) = local.with_year(new_year) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::YearDecimal => {
                let new_year = local.year() / 100 * 100 + local.year() % 10 + digit * 10;
                if let Some(changed) = local.with_year(new_year) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::YearUnit => {
                let new_year = local.year() / 10 * 10 + digit;
                if let Some(changed) = local.with_year(new_year) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::MonthDecimal => {
                let new_month = local.month() as i32 % 10 + digit * 10;
                if let Some(changed) = local.with_month(new_month as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::MonthUnit => {
                let new_month = local.month() as i32 / 10 * 10 + digit;
                if let Some(changed) = local.with_month(new_month as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::DayDecimal => {
                let new_day = local.day() as i32 % 10 + digit * 10;
                if let Some(changed) = local.with_day(new_day as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::DayUnit => {
                let new_day = local.day() as i32 / 10 * 10 + digit;
                if let Some(changed) = local.with_day(new_day as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::HourDecimal => {
                let new_hour = local.hour() as i32 % 10 + digit * 10;
                if let Some(changed) = local.with_hour(new_hour as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::HourUnit => {
                let new_hour = local.hour() as i32 / 10 * 10 + digit;
                if let Some(changed) = local.with_hour(new_hour as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::MinuteDecimal => {
                let new_minute = local.minute() as i32 % 10 + digit * 10;
                if let Some(changed) = local.with_minute(new_minute as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
            DateInputCursor::MuniteUnit => {
                let new_minute = local.minute() as i32 / 10 * 10 + digit;
                if let Some(changed) = local.with_minute(new_minute as u32) {
                    local = changed;
                    true
                } else {
                    false
                }
            }
        };
        if changed {
            self.cursor = self.cursor.next();
            self.date = local.to_utc();
        }
    }

    fn move_cursor_to_back(&mut self) {
        self.cursor = self.cursor.prev();
    }
}
