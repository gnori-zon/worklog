use uuid::Uuid;

use crate::repository::ProjectRepository;
use crate::{
    Service, models, repository,
    view_model::{
        Cursor, Msg, Notification, NotificationLevel, TextInput, create_message,
        entries::EntriesMsg,
    },
};

pub enum ProjectsMsg {
    SelectNext,
    SelectPrev,
    Create,
    Delete,
    Rename,
    Search,
    Input(char),
    Backspace,
    Cancel,
    Submit,
}

pub struct ProjectsModel {
    pub values: Vec<models::Project>,
    pub query: repository::ProjectQuery,
    pub paging: repository::Paging,
    pub cursor: Cursor,
    pub mode: ProjectModelMode,
}

#[derive(Clone)]
pub enum ProjectModelMode {
    View(Option<Notification>),
    Rename(TextInput),
    Create(TextInput),
    Search(TextInput),
    Delete(TextInput),
}

impl ProjectsModel {
    pub fn handle_msg<S: Service>(&mut self, msg: ProjectsMsg, service: &S) -> Option<Msg> {
        let mut next_msg: Option<Msg> = None;
        match msg {
            ProjectsMsg::SelectNext => {
                self.cursor.increment(self.values.len());
                next_msg = Some(Msg::Entries(EntriesMsg::ChangedProject(
                    self.get_selected().unwrap().id,
                )));
            }
            ProjectsMsg::SelectPrev => {
                self.cursor.decrement(self.values.len());
                next_msg = Some(Msg::Entries(EntriesMsg::ChangedProject(
                    self.get_selected().unwrap().id,
                )));
            }
            ProjectsMsg::Create => self.change_mode(ProjectModelMode::Create(TextInput::default())),
            ProjectsMsg::Delete => self.change_mode(ProjectModelMode::Delete(TextInput::default())),
            ProjectsMsg::Rename => {
                let text_input = TextInput {
                    text: self.get_selected().unwrap().name,
                };
                self.change_mode(ProjectModelMode::Rename(text_input));
            }
            ProjectsMsg::Input(ch) => self.handle_new_input(ch),
            ProjectsMsg::Backspace => self.handle_backspace(),
            ProjectsMsg::Cancel => self.change_mode(ProjectModelMode::View(None)),
            ProjectsMsg::Submit => self.handle_submit(service),
            ProjectsMsg::Search => self.change_mode(ProjectModelMode::Search(TextInput::default())),
        };
        next_msg
    }

    pub fn get_selected(&self) -> Option<models::Project> {
        self.cursor
            .selected
            .and_then(|index| self.values.get(index))
            .map(|project| project.clone())
    }

    fn change_mode(&mut self, mode: ProjectModelMode) {
        self.mode = mode
    }

    fn handle_new_input(&mut self, ch: char) {
        match &mut self.mode {
            ProjectModelMode::Rename(input)
            | ProjectModelMode::Create(input)
            | ProjectModelMode::Search(input)
            | ProjectModelMode::Delete(input) => input.push(ch),
            _ => {}
        }
    }

    fn handle_backspace(&mut self) {
        match &mut self.mode {
            ProjectModelMode::Rename(input)
            | ProjectModelMode::Create(input)
            | ProjectModelMode::Search(input)
            | ProjectModelMode::Delete(input) => input.backspace(),
            _ => {}
        }
    }

    fn handle_submit<S: Service>(&mut self, service: &S) {
        let notification: Option<Notification> = match &self.mode {
            ProjectModelMode::Rename(input) => self.handle_submit_rename(service, input.clone()),
            ProjectModelMode::Create(input) => self.handle_submit_create(service, input.clone()),
            ProjectModelMode::Search(input) => self.handle_submit_search(service, input.clone()),
            ProjectModelMode::Delete(input) => self.handle_submit_delete(service, input.clone()),
            _ => return,
        };
        self.change_mode(ProjectModelMode::View(notification));
    }

    fn handle_submit_search<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        match self.query.clone().name {
            repository::ProjectNameQuery::Any => {
                if input.text.is_empty() {
                    None
                } else {
                    self.query = repository::ProjectQuery {
                        name: repository::ProjectNameQuery::Contains(input.text.to_string()),
                    };
                    self.reload_values(service);
                    Some(Notification {
                        text: format!("Success search: '{}'", input.text),
                        level: NotificationLevel::Success,
                    })
                }
            }
            repository::ProjectNameQuery::Contains(old_input) => {
                if old_input == input.text {
                    None
                } else {
                    self.query = if input.text.is_empty() {
                        repository::ProjectQuery {
                            name: repository::ProjectNameQuery::Any,
                        }
                    } else {
                        repository::ProjectQuery {
                            name: repository::ProjectNameQuery::Contains(input.text.to_string()),
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

    fn handle_submit_create<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        let name = input.text.trim().to_string();
        let result = service.project_repository().create(models::Project {
            id: Uuid::now_v7(),
            name: name.clone(),
            archived: false,
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

    fn handle_submit_rename<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        let new_name = input.text.trim();
        let project = self.values.get_mut(self.cursor.selected.unwrap()).unwrap();
        let old_name = project.name.clone();
        let mut result: repository::Result<()> = Ok(());
        if old_name != new_name {
            let mut updated = project.clone();
            updated.name = String::from(new_name);
            result = service.project_repository().update(updated);
        }
        let notification = match result {
            Ok(_) => {
                project.name = String::from(new_name);
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

    fn handle_submit_delete<S: Service>(
        &mut self,
        service: &S,
        input: TextInput,
    ) -> Option<Notification> {
        let confirm_name = input.text.trim();
        let project = self.values.get(self.cursor.selected.unwrap()).unwrap();
        let current_name = project.name.clone();
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

    fn reload_values<S: Service>(&mut self, service: &S) {
        self.values = service
            .project_repository()
            .list(self.query.clone(), self.paging.clone())
            .unwrap();
        self.cursor.reset(self.values.len());
    }

    fn delete_selected<S: Service>(&mut self, service: &S) -> Option<Notification> {
        match self.cursor.selected {
            Some(index) => {
                let deleted = self.values.get(index).unwrap().clone();
                let result = service.project_repository().delete(deleted.id);
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
}
