use chrono::Utc;
use uuid::Uuid;

use crate::repository::EntryRepository;
use crate::{
    Service, models, repository,
    view_model::{Cursor, Msg, Notification, NotificationLevel, TextInput, create_message},
};

pub enum EntriesMsg {
    SelectNext,
    SelectPrev,
    Create(models::ProjectId),
    Delete,
    Rename,
    Search(Option<models::ProjectId>),
    Input(char),
    Backspace,
    Cancel,
    Submit,
    FinishEntry,
    ClearFinishEntry,
    ChangedProject(models::ProjectId),
}

pub struct EntriesModel {
    pub values: Vec<models::Entry>,
    pub paging: repository::Paging,
    pub query: repository::EntryQuery,
    pub cursor: Cursor,
    pub mode: EntriesModelMode,
}

#[derive(Clone)]
pub enum EntriesModelMode {
    View(Option<Notification>),
    Rename(TextInput),
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
            EntriesMsg::Cancel => self.change_mode(EntriesModelMode::View(None)),
            EntriesMsg::Submit => self.handle_submit(service),
            EntriesMsg::Search(project_id) => self.change_mode(EntriesModelMode::Search(
                project_id.clone(),
                TextInput::default(),
            )),
            EntriesMsg::FinishEntry => self.change_ended_at_entry(Some(Utc::now()), service),
            EntriesMsg::ClearFinishEntry => self.change_ended_at_entry(None, service),
        };
        None
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
                        text: format!("Failed delete {}: {}", deleted.name, create_message(err)),
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
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        match &mut self.mode {
            EntriesModelMode::Rename(input)
            | EntriesModelMode::Create(_, input)
            | EntriesModelMode::Search(_, input)
            | EntriesModelMode::Delete(input) => input.backspace(),
            _ => {}
        }
    }

    fn handle_submit<S: Service>(&mut self, service: &S) {
        let notification: Option<Notification> = match &self.mode {
            EntriesModelMode::Rename(input) => self.handle_submit_rename(service, input.clone()),
            EntriesModelMode::Create(project_id, input) => {
                self.handle_submit_create(service, project_id.clone(), input.clone())
            }
            EntriesModelMode::Search(project_id, input) => {
                self.handle_submit_search(service, project_id.clone(), input.clone())
            }
            EntriesModelMode::Delete(input) => self.handle_submit_delete(service, input.clone()),
            _ => return,
        };
        self.change_mode(EntriesModelMode::View(notification));
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
                    create_message(err)
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
                text: format!("Failed create {}: {}", name, create_message(err)),
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
        match self.query.clone().name {
            repository::EntryNameQuery::Any => {
                if input.text.is_empty() {
                    None
                } else {
                    self.query = repository::EntryQuery {
                        project: project_query,
                        name: repository::EntryNameQuery::Contains(input.text.to_string()),
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
                        }
                    } else {
                        repository::EntryQuery {
                            project: project_query,
                            name: repository::EntryNameQuery::Contains(input.text.to_string()),
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
                            create_message(err)
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
