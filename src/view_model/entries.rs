use chrono::Utc;
use uuid::Uuid;

use crate::repository::{EntryDateQuery, EntryRepository};
use crate::view_model::{DateInput, DateInputCursor};
use crate::{
    Service, models, repository,
    view_model::{Cursor, Msg, Notification, NotificationLevel, TextInput, create_message},
};

pub enum EntriesMsg {
    SelectNext,
    SelectPrev,
    SelectNextFilter,
    SelectPrevFilter,
    Create(models::ProjectId),
    Delete,
    Rename,
    Edit(EditEntryMsg),
    Search(Option<models::ProjectId>),
    Input(char),
    Backspace,
    Cancel,
    Submit,
    FinishEntry,
    ClearFinishEntry,
    ChangedProject(models::ProjectId),
}

pub enum EditEntryMsg {
    Start,
    SelectNextField,
    SelectPrevField,
    EditField,
}

pub struct EntriesModel {
    pub values: Vec<models::Entry>,
    pub paging: repository::Paging,
    pub query: repository::EntryQuery,
    pub cursor: Cursor,
    pub mode: EntriesModelMode,
}

#[derive(Clone)]
pub enum EntryEditModelMode {
    View(EntryEditModelView),
    EditStartedAt(DateInput),
    EditEndedAt(DateInput),
}

#[derive(Clone)]
pub enum EntryEditField {
    StartedAt,
    EndedAt,
}

impl EntryEditField {
    fn next(&self) -> Self {
        match self {
            EntryEditField::StartedAt => EntryEditField::EndedAt,
            EntryEditField::EndedAt => EntryEditField::EndedAt,
        }
    }

    fn prev(&self) -> Self {
        match self {
            EntryEditField::StartedAt => EntryEditField::StartedAt,
            EntryEditField::EndedAt => EntryEditField::StartedAt,
        }
    }
}

#[derive(Clone)]
pub struct EntryEditModelView {
    pub selected_field: EntryEditField,
    pub notification: Option<Notification>,
}

impl EntryEditModelView {
    fn select_next_field(&mut self) {
        self.selected_field = self.selected_field.next()
    }

    fn select_prev_field(&mut self) {
        self.selected_field = self.selected_field.prev()
    }
}

#[derive(Clone)]
pub enum EntriesModelMode {
    View(Option<Notification>),
    Rename(TextInput),
    Edit(models::Entry, EntryEditModelMode),
    Create(models::ProjectId, TextInput),
    Delete(TextInput),
    Search(Option<models::ProjectId>, TextInput),
}

impl EntriesModel {
    pub fn handle_msg<S: Service>(&mut self, msg: EntriesMsg, service: &S) -> Option<Msg> {
        match msg {
            EntriesMsg::SelectNext => self.cursor.increment(self.values.len()),
            EntriesMsg::SelectPrev => self.cursor.decrement(self.values.len()),
            EntriesMsg::ChangedProject(project_id) => match self.query.project {
                repository::EntryProjectQuery::Any => {
                    self.query.project = repository::EntryProjectQuery::In(project_id);
                    self.reload_values(service);
                }
                repository::EntryProjectQuery::In(exist_project_id)
                    if exist_project_id != project_id =>
                {
                    self.query.project = repository::EntryProjectQuery::In(project_id);
                    self.reload_values(service);
                }
                _ => {}
            },
            EntriesMsg::Create(project_id) => self.change_mode(EntriesModelMode::Create(
                project_id.clone(),
                TextInput::default(),
            )),
            EntriesMsg::Delete => self.change_mode(EntriesModelMode::Delete(TextInput::default())),
            EntriesMsg::Rename => {
                let text_input = TextInput {
                    text: self.get_selected().unwrap().name,
                };
                self.change_mode(EntriesModelMode::Rename(text_input));
            }
            EntriesMsg::Input(ch) => self.handle_new_input(ch),
            EntriesMsg::Backspace => self.handle_backspace(),
            EntriesMsg::Cancel => self.handle_cancel(),
            EntriesMsg::Submit => self.handle_submit(service),
            EntriesMsg::Search(project_id) => self.change_mode(EntriesModelMode::Search(
                project_id.clone(),
                TextInput::default(),
            )),
            EntriesMsg::FinishEntry => {
                self.change_ended_at_entry(Some(Utc::now()), service);
                self.reload_values(service);
            }
            EntriesMsg::ClearFinishEntry => self.change_ended_at_entry(None, service),
            EntriesMsg::SelectNextFilter => {
                self.query.date = self.query.date.next();
                self.reload_values(service);
            }
            EntriesMsg::SelectPrevFilter => {
                self.query.date = self.query.date.prev();
                self.reload_values(service);
            }
            EntriesMsg::Edit(edit_entry_msg) => match edit_entry_msg {
                EditEntryMsg::Start => self.change_mode(EntriesModelMode::Edit(
                    self.get_selected().unwrap().clone(),
                    EntryEditModelMode::View(EntryEditModelView {
                        selected_field: EntryEditField::StartedAt,
                        notification: None,
                    }),
                )),
                EditEntryMsg::SelectPrevField | EditEntryMsg::SelectNextField => {
                    if let EntriesModelMode::Edit(_, edit_mode) = &mut self.mode
                        && let EntryEditModelMode::View(entry_edit_model_view) = edit_mode
                    {
                        match edit_entry_msg {
                            EditEntryMsg::SelectNextField => {
                                entry_edit_model_view.select_next_field()
                            }
                            EditEntryMsg::SelectPrevField => {
                                entry_edit_model_view.select_prev_field()
                            }
                            _ => {}
                        }
                    }
                }
                EditEntryMsg::EditField => {
                    if let EntriesModelMode::Edit(update_state, edit_mode) = &mut self.mode
                        && let EntryEditModelMode::View(entry_edit_model_view) = edit_mode
                    {
                        let update_state = update_state.clone();
                        let new_mode = match entry_edit_model_view.selected_field {
                            EntryEditField::StartedAt => {
                                let current_started_at = update_state.started_at.clone();
                                EntryEditModelMode::EditStartedAt(DateInput {
                                    date: current_started_at,
                                    cursor: DateInputCursor::default(),
                                })
                            }
                            EntryEditField::EndedAt => {
                                let current_ended_at =
                                    update_state.ended_at.clone().unwrap_or_else(|| Utc::now());
                                EntryEditModelMode::EditEndedAt(DateInput {
                                    date: current_ended_at,
                                    cursor: DateInputCursor::default(),
                                })
                            }
                        };
                        self.change_mode(EntriesModelMode::Edit(update_state, new_mode))
                    }
                }
            },
        };
        None
    }

    fn handle_cancel(&mut self) {
        match &self.mode {
            EntriesModelMode::View(_)
            | EntriesModelMode::Rename(_)
            | EntriesModelMode::Create(_, _)
            | EntriesModelMode::Delete(_)
            | EntriesModelMode::Search(_, _) => self.change_mode(EntriesModelMode::View(None)),
            EntriesModelMode::Edit(update_state, edit_mode) => match edit_mode {
                EntryEditModelMode::View(_) => self.change_mode(EntriesModelMode::View(None)),
                EntryEditModelMode::EditStartedAt(_) | EntryEditModelMode::EditEndedAt(_) => {
                    let selected_field = match edit_mode {
                        EntryEditModelMode::EditStartedAt(_) => EntryEditField::StartedAt,
                        EntryEditModelMode::EditEndedAt(_) => EntryEditField::EndedAt,
                        _ => panic!(
                            "unexpected state: expected only editName, editStartedAt, editEndedAt"
                        ),
                    };
                    self.change_mode(EntriesModelMode::Edit(
                        update_state.clone(),
                        EntryEditModelMode::View(EntryEditModelView {
                            selected_field: selected_field,
                            notification: None,
                        }),
                    ))
                }
            },
        }
    }

    pub fn get_selected(&self) -> Option<models::Entry> {
        self.cursor
            .selected
            .and_then(|index| self.values.get(index))
            .map(|entry| entry.clone())
    }

    fn change_mode(&mut self, mode: EntriesModelMode) {
        self.mode = mode
    }

    fn reload_values<S: Service>(&mut self, service: &S) {
        self.values = service
            .entry_repository()
            .list(self.query.clone(), self.paging.clone())
            .unwrap();
        self.cursor.reset(self.values.len());
    }

    fn delete_selected<S: Service>(&mut self, service: &S) -> Option<Notification> {
        match self.cursor.selected {
            Some(index) => {
                let deleted = self.values.get(index).unwrap().clone();
                let result = service.entry_repository().delete(deleted.id);
                if result.is_ok() {
                    self.reload_values(service);
                }
                let notification = match result {
                    Ok(_) => Notification {
                        text: format!("Success deleted {}", deleted.name),
                        level: NotificationLevel::Success,
                    },
                    Err(err) => Notification {
                        text: format!("Failed delete {}: {}", deleted.name, create_message(&err)),
                        level: NotificationLevel::Failure,
                    },
                };
                Some(notification)
            }
            None => None,
        }
    }

    fn handle_new_input(&mut self, ch: char) {
        match &mut self.mode {
            EntriesModelMode::Rename(input)
            | EntriesModelMode::Create(_, input)
            | EntriesModelMode::Search(_, input)
            | EntriesModelMode::Delete(input) => input.push(ch),
            EntriesModelMode::Edit(_, edit_mode) => match edit_mode {
                EntryEditModelMode::EditStartedAt(input)
                | EntryEditModelMode::EditEndedAt(input) => input.push_local_tz(ch),
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        match &mut self.mode {
            EntriesModelMode::Rename(input)
            | EntriesModelMode::Create(_, input)
            | EntriesModelMode::Search(_, input)
            | EntriesModelMode::Delete(input) => input.backspace(),
            EntriesModelMode::Edit(_, edit_mode) => match edit_mode {
                EntryEditModelMode::EditStartedAt(input)
                | EntryEditModelMode::EditEndedAt(input) => input.move_cursor_to_back(),
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_submit<S: Service>(&mut self, service: &S) {
        match &self.mode {
            EntriesModelMode::Rename(input) => {
                let notification = self.handle_submit_rename(service, input.clone());
                self.change_mode(EntriesModelMode::View(notification));
            }
            EntriesModelMode::Create(project_id, input) => {
                let notification =
                    self.handle_submit_create(service, project_id.clone(), input.clone());
                self.change_mode(EntriesModelMode::View(notification));
            }
            EntriesModelMode::Search(project_id, input) => {
                let notification =
                    self.handle_submit_search(service, project_id.clone(), input.clone());
                self.change_mode(EntriesModelMode::View(notification));
            }
            EntriesModelMode::Delete(input) => {
                let notification = self.handle_submit_delete(service, input.clone());
                self.change_mode(EntriesModelMode::View(notification));
            }
            EntriesModelMode::Edit(update_state, edit_mode) => match edit_mode {
                EntryEditModelMode::View(_) => {
                    let entry = self.values.get_mut(self.cursor.selected.unwrap()).unwrap();
                    let mut result: repository::Result<()> = Ok(());
                    if entry.started_at.ne(&update_state.started_at)
                        || entry.ended_at.ne(&update_state.ended_at)
                    {
                        let mut updated = entry.clone();
                        updated.started_at = update_state.started_at;
                        updated.ended_at = update_state.ended_at;
                        result = service.entry_repository().update(updated);
                    }
                    let notification = match &result {
                        Ok(_) => {
                            entry.started_at = update_state.started_at;
                            entry.ended_at = update_state.ended_at;
                            Notification {
                                text: format!("Success edit {}", update_state.name),
                                level: NotificationLevel::Success,
                            }
                        }
                        Err(err) => Notification {
                            text: format!(
                                "Failed edit {}: {}",
                                update_state.name,
                                create_message(err)
                            ),
                            level: NotificationLevel::Failure,
                        },
                    };
                    if result.is_ok() {
                        self.reload_values(service);
                    }
                    self.change_mode(EntriesModelMode::View(Some(notification)));
                }
                EntryEditModelMode::EditStartedAt(date_input) => {
                    let new_started_at = date_input.date;
                    if let Some(ended_at) = update_state.ended_at
                        && ended_at.lt(&new_started_at)
                    {
                        let notification = Notification {
                            text: String::from(
                                "Failed change started_at: ended_at must be less than started_at",
                            ),
                            level: NotificationLevel::Failure,
                        };
                        self.change_mode(EntriesModelMode::Edit(
                            update_state.clone(),
                            EntryEditModelMode::View(EntryEditModelView {
                                selected_field: EntryEditField::StartedAt,
                                notification: Some(notification),
                            }),
                        ));
                    } else {
                        let mut new_update_state = update_state.clone();
                        new_update_state.started_at = new_started_at;
                        let notification = Notification {
                            text: String::from("Success change started_at"),
                            level: NotificationLevel::Success,
                        };
                        self.change_mode(EntriesModelMode::Edit(
                            new_update_state,
                            EntryEditModelMode::View(EntryEditModelView {
                                selected_field: EntryEditField::StartedAt,
                                notification: Some(notification),
                            }),
                        ));
                    };
                }
                EntryEditModelMode::EditEndedAt(date_input) => {
                    let new_ended_at = date_input.date;
                    if new_ended_at.lt(&update_state.started_at) {
                        let notification = Notification {
                            text: String::from(
                                "Failed change ended_at: ended_at must be less than started_at",
                            ),
                            level: NotificationLevel::Failure,
                        };
                        self.change_mode(EntriesModelMode::Edit(
                            update_state.clone(),
                            EntryEditModelMode::View(EntryEditModelView {
                                selected_field: EntryEditField::EndedAt,
                                notification: Some(notification),
                            }),
                        ));
                    } else {
                        let mut new_update_state = update_state.clone();
                        new_update_state.ended_at = Some(new_ended_at);
                        let notification = Notification {
                            text: String::from("Success change ended_at"),
                            level: NotificationLevel::Success,
                        };
                        self.change_mode(EntriesModelMode::Edit(
                            new_update_state,
                            EntryEditModelMode::View(EntryEditModelView {
                                selected_field: EntryEditField::EndedAt,
                                notification: Some(notification),
                            }),
                        ));
                    };
                }
            },
            EntriesModelMode::View(_) => return,
        };
    }

    fn handle_submit_rename<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        let new_name = input.text.trim();
        let entry = self.values.get_mut(self.cursor.selected.unwrap()).unwrap();
        let old_name = entry.name.clone();
        let mut result: repository::Result<()> = Ok(());
        if old_name != new_name {
            let mut updated = entry.clone();
            updated.name = String::from(new_name);
            result = service.entry_repository().update(updated);
        }
        let notification = match result {
            Ok(_) => {
                entry.name = String::from(new_name);
                Notification {
                    text: format!("Success rename {} to {}", old_name, new_name),
                    level: NotificationLevel::Success,
                }
            }
            Err(err) => Notification {
                text: format!(
                    "Failed rename {} to {}: {}",
                    old_name,
                    new_name,
                    create_message(&err)
                ),
                level: NotificationLevel::Failure,
            },
        };
        Some(notification)
    }

    fn handle_submit_create<S: Service>(
        &mut self,
        service: &S,
        project_id: models::ProjectId,
        input: TextInput,
    ) -> Option<Notification> {
        let name = input.text.trim().to_string();
        let result = service.entry_repository().create(models::Entry {
            id: Uuid::now_v7(),
            name: name.clone(),
            project_id: project_id,
            started_at: Utc::now(),
            ended_at: None,
        });
        self.reload_values(service);
        let notification = match result {
            Ok(_) => Notification {
                text: format!("Success create {}", name),
                level: NotificationLevel::Success,
            },
            Err(err) => Notification {
                text: format!("Failed create {}: {}", name, create_message(&err)),
                level: NotificationLevel::Failure,
            },
        };
        Some(notification)
    }

    fn handle_submit_search<S: Service>(
        &mut self,
        service: &S,
        project_id: Option<models::ProjectId>,
        input: TextInput,
    ) -> Option<Notification> {
        let project_query = match project_id {
            Some(project_id) => repository::EntryProjectQuery::In(project_id),
            None => repository::EntryProjectQuery::Any,
        };
        let date_query = self.query.date.clone();
        match self.query.clone().name {
            repository::EntryNameQuery::Any => {
                if input.text.is_empty() {
                    None
                } else {
                    self.query = repository::EntryQuery {
                        project: project_query,
                        name: repository::EntryNameQuery::Contains(input.text.to_string()),
                        date: date_query,
                    };
                    self.reload_values(service);
                    Some(Notification {
                        text: format!("Success search: '{}'", input.text),
                        level: NotificationLevel::Success,
                    })
                }
            }
            repository::EntryNameQuery::Contains(old_input) => {
                if old_input == input.text {
                    None
                } else {
                    self.query = if input.text.is_empty() {
                        repository::EntryQuery {
                            project: project_query,
                            name: repository::EntryNameQuery::Any,
                            date: date_query,
                        }
                    } else {
                        repository::EntryQuery {
                            project: project_query,
                            name: repository::EntryNameQuery::Contains(input.text.to_string()),
                            date: date_query,
                        }
                    };
                    self.reload_values(service);
                    Some(Notification {
                        text: format!("Success search: {}", input.text),
                        level: NotificationLevel::Success,
                    })
                }
            }
        }
    }

    fn handle_submit_delete<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        let confirm_name = input.text.trim();
        let entry = self.values.get(self.cursor.selected.unwrap()).unwrap();
        let current_name = entry.name.clone();
        if confirm_name != current_name {
            return Some(Notification {
                text: format!(
                    "Failed delete {}: failed confirmation",
                    current_name.clone()
                ),
                level: NotificationLevel::Failure,
            });
        }
        self.delete_selected(service)
    }

    fn change_ended_at_entry<S: Service>(
        &mut self,
        new_ended_at: Option<models::DateTime>,
        service: &S,
    ) {
        match &self.mode {
            EntriesModelMode::View(_) => {
                let entry = self.values.get_mut(self.cursor.selected.unwrap()).unwrap();
                let old_ended = entry.ended_at.clone();
                let mut result: repository::Result<()> = Ok(());
                if old_ended != new_ended_at {
                    let mut updated = entry.clone();
                    updated.ended_at = new_ended_at;
                    result = service.entry_repository().update(updated);
                }

                let operation = if new_ended_at.is_some() {
                    "finish"
                } else {
                    "clear finish"
                };
                let notification = match result {
                    Ok(_) => {
                        entry.ended_at = new_ended_at;
                        Notification {
                            text: format!("Success {} entry {}", operation, entry.name.clone()),
                            level: NotificationLevel::Success,
                        }
                    }
                    Err(err) => Notification {
                        text: format!(
                            "Failed {} entry {}: {}",
                            operation,
                            entry.name.clone(),
                            create_message(&err)
                        ),
                        level: NotificationLevel::Failure,
                    },
                };
                self.change_mode(EntriesModelMode::View(Some(notification)));
            }
            _ => return,
        };
    }
}

impl EntryDateQuery {
    pub fn all_cases() -> Vec<EntryDateQuery> {
        vec![
            EntryDateQuery::Running,
            EntryDateQuery::Today,
            EntryDateQuery::Week,
            EntryDateQuery::Mounth,
            EntryDateQuery::Any,
        ]
    }

    fn next(&self) -> EntryDateQuery {
        match &self {
            EntryDateQuery::Running => EntryDateQuery::Today,
            EntryDateQuery::Today => EntryDateQuery::Week,
            EntryDateQuery::Week => EntryDateQuery::Mounth,
            EntryDateQuery::Mounth => EntryDateQuery::Any,
            EntryDateQuery::Any => EntryDateQuery::Running,
        }
    }
    fn prev(&self) -> EntryDateQuery {
        match &self {
            EntryDateQuery::Running => EntryDateQuery::Any,
            EntryDateQuery::Today => EntryDateQuery::Running,
            EntryDateQuery::Week => EntryDateQuery::Today,
            EntryDateQuery::Mounth => EntryDateQuery::Week,
            EntryDateQuery::Any => EntryDateQuery::Mounth,
        }
    }
}
