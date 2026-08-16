pub mod json;

use crate::models::{Entry, EntryId, Project, ProjectId};

#[derive(Debug)]
pub enum Error {
    NotExistsById(String),
    NameAlreadyUsed(NameAlreadyUsed),
    IdAlreadyUsed(String),
    BrokenState(BrokenState),
    Json(serde_json::Error),
    Io(std::io::Error),
    Uuid(uuid::Error),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::Uuid(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

#[derive(Debug)]
pub struct NameAlreadyUsed {
    pub name: String,
    pub id: String,
}

#[derive(Debug)]
pub enum BrokenState {
    HasDuplicateIds(String),
    HasDuplicateNames(String),
}

pub type Result<V> = std::result::Result<V, Error>;

#[derive(Debug, Clone)]
pub struct Paging {
    offset: usize,
    limit: usize,
}

impl Default for Paging {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 100,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectQuery {
    pub name: ProjectNameQuery,
}

#[derive(Debug, Clone)]
pub enum ProjectNameQuery {
    Any,
    Contains(String),
}

impl Default for ProjectNameQuery {
    fn default() -> Self {
        ProjectNameQuery::Any
    }
}

pub trait ProjectRepository {
    fn total(&self, query: ProjectQuery) -> i64;
    fn list(&self, query: ProjectQuery, paging: Paging) -> Result<Vec<Project>>;
    fn create(&self, project: Project) -> Result<()>;
    fn update(&self, project: Project) -> Result<()>;
    fn delete(&self, id: ProjectId) -> Result<()>;
}

#[derive(Default, Debug, Clone)]
pub struct EntryQuery {
    pub name: EntryNameQuery,
    pub project: EntryProjectQuery,
}

#[derive(Debug, Clone)]
pub enum EntryNameQuery {
    Any,
    Contains(String),
}

impl Default for EntryNameQuery {
    fn default() -> Self {
        EntryNameQuery::Any
    }
}

#[derive(Debug, Clone)]
pub enum EntryProjectQuery {
    Any,
    In(ProjectId),
}

impl Default for EntryProjectQuery {
    fn default() -> Self {
        EntryProjectQuery::Any
    }
}

pub trait EntryRepository {
    fn total(&self, query: EntryQuery) -> i64;
    fn list(&self, query: EntryQuery, paging: Paging) -> Result<Vec<Entry>>;
    fn create(&self, entry: Entry) -> Result<()>;
    fn update(&self, entry: Entry) -> Result<()>;
    fn delete(&self, id: EntryId) -> Result<()>;
}

#[cfg(test)]
pub mod contract {
    use std::{collections::HashSet, ops::Add};

    use crate::{
        models::{Entry, Project},
        repository::{
            EntryNameQuery,
            EntryProjectQuery::{self},
            EntryQuery, EntryRepository,
            Error::{IdAlreadyUsed, NameAlreadyUsed, NotExistsById},
            Paging, ProjectNameQuery, ProjectQuery, ProjectRepository,
        },
    };
    use chrono::Utc;
    use uuid::Uuid;

    pub fn should_sucess_create_project_when_model_is_valid<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let query = ProjectQuery {
            name: ProjectNameQuery::Any,
        };

        let result = repository.create(create_project.clone());
        let found_projects = repository.list(query.clone(), Paging::default());
        let total_result = repository.total(query.clone());

        assert!(result.is_ok());
        assert!(found_projects.is_ok());
        let found_projects = found_projects.unwrap();
        assert_eq!(found_projects.len(), 1);
        assert_eq!(found_projects.first().unwrap().clone(), create_project);
        assert_eq!(total_result, 1);
    }

    pub fn should_return_err_when_create_project_with_duplicate_id<R: ProjectRepository>(
        repository: &R,
    ) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let duplicate_create_project = Project {
            id: id,
            name: id.to_string().add("_new_postfix"),
            archived: false,
        };

        let result = repository.create(create_project.clone());
        let duplicate_result = repository.create(duplicate_create_project);
        let found_projects = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging::default(),
        );

        assert!(result.is_ok());
        assert!(duplicate_result.is_err());
        assert!(
            matches!(duplicate_result.unwrap_err(), IdAlreadyUsed(value) if value == id.to_string())
        );
        assert!(found_projects.is_ok());
        let found_projects = found_projects.unwrap();
        assert_eq!(found_projects.len(), 1);
        assert_eq!(found_projects.first().unwrap().clone(), create_project);
    }

    pub fn should_return_err_when_create_project_with_duplicate_name<R: ProjectRepository>(
        repository: &R,
    ) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let duplicate_create_project = Project {
            id: Uuid::now_v7(),
            name: id.to_string(),
            archived: false,
        };

        let result = repository.create(create_project.clone());
        let duplicate_result = repository.create(duplicate_create_project);
        let found_projects = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging::default(),
        );

        assert!(result.is_ok());
        assert!(duplicate_result.is_err());
        assert!(
            matches!(duplicate_result.unwrap_err(), NameAlreadyUsed(value) if value.id == id.to_string() && value.name == id.to_string())
        );
        assert!(found_projects.is_ok());

        let found_projects = found_projects.unwrap();
        assert_eq!(found_projects.len(), 1);
        assert_eq!(found_projects.first().unwrap().clone(), create_project);
        assert_eq!(
            repository.total(ProjectQuery {
                name: ProjectNameQuery::Any
            }),
            1
        );
    }

    pub fn should_update_where_project_exist<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let update_project = Project {
            id: id,
            name: create_project.name.clone().add("_new_postfix"),
            archived: true,
        };
        let create_result = repository.create(create_project);
        let update_result = repository.update(update_project.clone());
        let found_projects = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging::default(),
        );

        assert!(create_result.is_ok());
        assert!(update_result.is_ok());
        assert!(found_projects.is_ok());
        let found_projects = found_projects.unwrap();
        assert_eq!(found_projects.len(), 1);
        assert_eq!(found_projects.first().unwrap().clone(), update_project);
    }

    pub fn should_return_err_when_update_not_exist_project<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let update_project = Project {
            id: id,
            name: id.to_string(),
            archived: true,
        };

        let update_result = repository.update(update_project);

        assert!(update_result.is_err());
        assert!(
            matches!(update_result.unwrap_err(), NotExistsById(value) if value == id.to_string())
        )
    }

    pub fn should_return_err_when_update_project_with_already_used_name<R: ProjectRepository>(
        repository: &R,
    ) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let second_id = Uuid::now_v7();
        let second_create_project = Project {
            id: second_id,
            name: second_id.to_string(),
            archived: false,
        };
        let update_project = Project {
            id: second_create_project.id.clone(),
            name: create_project.name.clone(),
            archived: true,
        };

        let create_result = repository.create(create_project.clone());
        let second_create_result = repository.create(second_create_project.clone());
        let update_result = repository.update(update_project);

        assert!(create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert!(update_result.is_err());
        assert!(
            matches!(update_result.unwrap_err(), NameAlreadyUsed(value) if value.id == create_project.id.to_string() && value.name == create_project.name)
        )
    }

    pub fn should_delete_where_project_exists<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let create_project = Project {
            id: id,
            name: id.to_string(),
            archived: false,
        };
        let create_result = repository.create(create_project);
        let delete_result = repository.delete(id);
        let found_projects = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging::default(),
        );
        let total_after_delete = repository.total(ProjectQuery {
            name: ProjectNameQuery::Any,
        });

        assert!(create_result.is_ok());
        assert!(delete_result.is_ok());
        assert!(found_projects.is_ok());
        let found_projects = found_projects.unwrap();
        assert!(found_projects.is_empty());
        assert_eq!(total_after_delete, 0);
    }

    pub fn should_return_err_where_delete_not_exist_project<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let delete_result = repository.delete(id);

        assert!(delete_result.is_err());
        assert!(
            matches!(delete_result.unwrap_err(), NotExistsById(value) if value == id.to_string())
        )
    }

    pub fn should_return_total_projects<R: ProjectRepository>(repository: &R) {
        let total_before_create = repository.total(ProjectQuery {
            name: ProjectNameQuery::Any,
        });

        let first_create_result = repository.create(Project {
            id: Uuid::now_v7(),
            name: "first_name".to_string(),
            archived: false,
        });
        let second_create_result = repository.create(Project {
            id: Uuid::now_v7(),
            name: "second_name".to_string(),
            archived: false,
        });

        let total_after_create = repository.total(ProjectQuery {
            name: ProjectNameQuery::Any,
        });
        let total_filtered_by_first_substr = repository.total(ProjectQuery {
            name: ProjectNameQuery::Contains("first".to_string()),
        });
        let total_filtered_by_second_substr = repository.total(ProjectQuery {
            name: ProjectNameQuery::Contains("second".to_string()),
        });

        assert_eq!(total_before_create, 0);
        assert!(first_create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert_eq!(total_after_create, 2);
        assert_eq!(total_filtered_by_first_substr, 1);
        assert_eq!(total_filtered_by_second_substr, 1);
    }

    pub fn should_return_empty_projects_when_limit_is_zero<R: ProjectRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let crete_result = repository.create(Project {
            id: id,
            name: id.to_string().add("_name"),
            archived: false,
        });

        let list_with_not_zero_limit_result = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 1,
            },
        );
        let list_with_zero_limit_result = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 0,
            },
        );

        assert!(crete_result.is_ok());
        assert!(list_with_not_zero_limit_result.is_ok());
        assert!(list_with_not_zero_limit_result.unwrap().len() > 0);
        assert!(list_with_zero_limit_result.is_ok());
        assert!(list_with_zero_limit_result.unwrap().is_empty());
    }

    pub fn should_return_valid_projects_when_limit_is_not_zero<R: ProjectRepository>(
        repository: &R,
    ) {
        let id = Uuid::now_v7();
        let crete_result = repository.create(Project {
            id: id,
            name: id.to_string().add("_name"),
            archived: false,
        });

        let list_with_limit_eq_size = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 1,
            },
        );
        let list_with_limit_gt_size = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 5,
            },
        );

        assert!(crete_result.is_ok());
        assert!(list_with_limit_eq_size.is_ok());
        assert_eq!(list_with_limit_eq_size.unwrap().len(), 1);
        assert!(list_with_limit_gt_size.is_ok());
        assert_eq!(list_with_limit_gt_size.unwrap().len(), 1);
    }

    pub fn should_can_paginate_by_projects_using_offset_and_limit<R: ProjectRepository>(
        repository: &R,
    ) {
        let empty_query = ProjectQuery {
            name: ProjectNameQuery::Any,
        };
        let project_ids: HashSet<Uuid> = (0..5).map(|_| Uuid::now_v7()).collect();
        let projects: Vec<Project> = project_ids
            .iter()
            .map(|id| Project {
                id: id.clone(),
                name: id.to_string().add("_name"),
                archived: false,
            })
            .collect();
        let create_project_results: Vec<super::Result<()>> = projects
            .iter()
            .cloned()
            .map(|project| repository.create(project))
            .collect();

        let first_page = repository.list(
            empty_query.clone(),
            Paging {
                offset: 0,
                limit: 2,
            },
        );
        let second_page = repository.list(
            empty_query.clone(),
            Paging {
                offset: 2,
                limit: 3,
            },
        );
        let third_page = repository.list(
            empty_query.clone(),
            Paging {
                offset: 6,
                limit: 5,
            },
        );

        create_project_results
            .iter()
            .for_each(|result| assert!(result.is_ok()));

        assert!(first_page.is_ok());
        let first_page = first_page.unwrap();
        let first_page_ids: HashSet<Uuid> = first_page.iter().map(|project| project.id).collect();
        assert_eq!(first_page.len(), 2);
        assert_eq!(first_page_ids.len(), first_page.len());
        assert!(project_ids.is_superset(&first_page_ids));

        assert!(second_page.is_ok());
        let second_page = second_page.unwrap();
        let second_page_ids: HashSet<Uuid> = second_page.iter().map(|project| project.id).collect();
        assert_eq!(second_page.len(), 3);
        assert_eq!(second_page_ids.len(), second_page.len());
        assert!(project_ids.is_superset(&second_page_ids));
        second_page_ids.iter().for_each(|id| print!("{}", id));
        println!("");
        first_page_ids.iter().for_each(|id| print!("{}", id));
        assert!(second_page_ids.is_disjoint(&first_page_ids));

        assert!(third_page.is_ok());
        assert!(third_page.unwrap().is_empty());

        let mut total_pages = first_page.clone();
        total_pages.append(&mut second_page.clone());
        let mut sorted_pages = total_pages.clone();
        sorted_pages.sort_by(|a, b| a.id.cmp(&b.id));

        let total_pages_ids: Vec<Uuid> = total_pages
            .iter()
            .map(|project| project.id.clone())
            .collect();
        let sorted_pages_ids: Vec<Uuid> = sorted_pages
            .iter()
            .map(|project| project.id.clone())
            .collect();

        assert_eq!(total_pages_ids, sorted_pages_ids);
    }

    pub fn should_can_filter_projects_by_name<R: ProjectRepository>(repository: &R) {
        let paging = Paging {
            offset: 0,
            limit: 5,
        };
        let empty_query = ProjectQuery {
            name: ProjectNameQuery::Any,
        };
        let list_all_before_create = repository.list(empty_query.clone(), paging.clone());

        let first_create_result = repository.create(Project {
            id: Uuid::now_v7(),
            name: "first_name".to_string(),
            archived: false,
        });
        let second_create_result = repository.create(Project {
            id: Uuid::now_v7(),
            name: "second_name".to_string(),
            archived: false,
        });

        let list_all_after_create = repository.list(empty_query.clone(), paging.clone());
        let list_filtered_by_first_substr = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Contains("first".to_string()),
            },
            paging.clone(),
        );
        let list_filtered_by_second_substr = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Contains("second".to_string()),
            },
            paging.clone(),
        );
        let list_filtered_by_common_infix = repository.list(
            ProjectQuery {
                name: ProjectNameQuery::Contains("nam".to_string()),
            },
            paging.clone(),
        );

        assert!(list_all_before_create.is_ok());
        let list_all_before_create = list_all_before_create.unwrap();
        assert!(list_all_before_create.is_empty());
        assert!(first_create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert!(list_all_after_create.is_ok());
        let list_all_after_create = list_all_after_create.unwrap();
        assert_eq!(list_all_after_create.len(), 2);
        assert!(list_filtered_by_first_substr.is_ok());
        let list_filtered_by_first_substr = list_filtered_by_first_substr.unwrap();
        assert_eq!(list_filtered_by_first_substr.len(), 1);
        assert!(list_filtered_by_second_substr.is_ok());
        let list_filtered_by_second_substr = list_filtered_by_second_substr.unwrap();
        assert_eq!(list_filtered_by_second_substr.len(), 1);
        assert!(list_filtered_by_common_infix.is_ok());
        let list_filtered_by_common_infix = list_filtered_by_common_infix.unwrap();
        assert_eq!(list_filtered_by_common_infix.len(), 2);
    }

    pub fn should_sucess_create_entry_when_model_is_valid<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let create_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };

        let result = repository.create(create_entry.clone());
        let found_entries = repository.list(query.clone(), Paging::default());
        let total_result = repository.total(query.clone());

        assert!(result.is_ok());
        assert!(found_entries.is_ok());
        let found_entries = found_entries.unwrap();
        assert_eq!(found_entries.len(), 1);
        assert_eq!(found_entries.first().unwrap().clone(), create_entry);
        assert_eq!(total_result, 1);
    }

    pub fn should_return_err_when_create_entry_with_duplicate_id<R: EntryRepository>(
        repository: &R,
    ) {
        let id = Uuid::now_v7();
        let create_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let duplicate_create_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string().add("_new_postfix"),
            started_at: Utc::now(),
            ended_at: None,
        };

        let result = repository.create(create_entry.clone());
        let duplicate_result = repository.create(duplicate_create_entry);
        let found_entries = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging::default(),
        );

        assert!(result.is_ok());
        assert!(duplicate_result.is_err());
        assert!(
            matches!(duplicate_result.unwrap_err(), IdAlreadyUsed(value) if value == id.to_string())
        );
        assert!(found_entries.is_ok());
        let found_entries = found_entries.unwrap();
        assert_eq!(found_entries.len(), 1);
        assert_eq!(found_entries.first().unwrap().clone(), create_entry);
    }

    pub fn should_return_err_when_create_project_with_duplicate_name_in_same_project<
        R: EntryRepository,
    >(
        repository: &R,
    ) {
        let name = Uuid::now_v7().to_string();
        let project_id = Uuid::now_v7();
        let create_entry = Entry {
            id: Uuid::now_v7(),
            project_id: project_id,
            name: name.clone(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let duplcate_create_entry_in_same_project = Entry {
            id: Uuid::now_v7(),
            project_id: project_id,
            name: name.clone(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let duplcate_create_entry_in_other_project = Entry {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            name: name.clone(),
            started_at: Utc::now(),
            ended_at: None,
        };

        let create_result = repository.create(create_entry.clone());
        let duplicate_create_entry_in_same_project_result =
            repository.create(duplcate_create_entry_in_same_project.clone());
        let duplicate_create_entry_in_other_project_result =
            repository.create(duplcate_create_entry_in_other_project.clone());
        let found_entries = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging::default(),
        );

        assert!(create_result.is_ok());
        assert!(duplicate_create_entry_in_other_project_result.is_ok());
        assert!(duplicate_create_entry_in_same_project_result.is_err());
        assert!(
            matches!(duplicate_create_entry_in_same_project_result.unwrap_err(), NameAlreadyUsed(value) if value.id == create_entry.id.clone().to_string() && value.name == create_entry.name.clone())
        );
        assert!(found_entries.is_ok());
        let found_entries = found_entries.unwrap();
        assert_eq!(found_entries.len(), 2);
        assert_eq!(found_entries.first().unwrap().clone(), create_entry);
    }

    pub fn should_update_where_entry_exist<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let create_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let update_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: create_entry.name.clone().add("_new_postfix"),
            started_at: create_entry.started_at.clone(),
            ended_at: Some(Utc::now()),
        };
        let create_result = repository.create(create_entry);
        let update_result = repository.update(update_entry.clone());
        let found_entries = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging::default(),
        );

        assert!(create_result.is_ok());
        assert!(update_result.is_ok());
        assert!(found_entries.is_ok());
        let found_projects = found_entries.unwrap();
        assert_eq!(found_projects.len(), 1);
        assert_eq!(found_projects.first().unwrap().clone(), update_entry);
    }

    pub fn should_return_err_when_update_not_exist_entry<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let update_entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string(),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
        };

        let update_result = repository.update(update_entry);
        let found_entries = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging::default(),
        );

        assert!(update_result.is_err());
        assert!(
            matches!(update_result.unwrap_err(), NotExistsById(value) if value == id.to_string())
        );
        assert!(found_entries.is_ok());
        assert!(found_entries.unwrap().is_empty());
    }

    pub fn should_return_err_when_update_entry_with_already_used_name_in_same_project<
        R: EntryRepository,
    >(
        repository: &R,
    ) {
        let project_id = Uuid::now_v7();
        let first_entry_id = Uuid::now_v7();
        let first_entry = Entry {
            id: first_entry_id,
            project_id: project_id,
            name: first_entry_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let second_entry_id = Uuid::now_v7();
        let second_entry = Entry {
            id: second_entry_id,
            project_id: project_id,
            name: second_entry_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let entry_in_other_project_id = Uuid::now_v7();
        let entry_in_other_project = Entry {
            id: entry_in_other_project_id,
            project_id: Uuid::now_v7(),
            name: entry_in_other_project_id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };

        let update_second_entry = Entry {
            name: first_entry.name.clone(),
            ..second_entry
        };
        let update_entry_in_other_project = Entry {
            name: first_entry.name.clone(),
            ..entry_in_other_project
        };

        let create_first_entry_result = repository.create(first_entry.clone());
        let create_second_entry_result = repository.create(second_entry);
        let create_entry_in_other_project_result = repository.create(entry_in_other_project);

        let update_second_entry_result = repository.update(update_second_entry);
        let update_entry_in_other_project_result = repository.update(update_entry_in_other_project);

        assert!(create_first_entry_result.is_ok());
        assert!(create_second_entry_result.is_ok());
        assert!(create_entry_in_other_project_result.is_ok());
        assert!(update_second_entry_result.is_err());
        assert!(
            matches!(update_second_entry_result.unwrap_err(), NameAlreadyUsed(value) if value.id == first_entry_id.to_string() && value.name == first_entry.name.clone())
        );
        assert!(update_entry_in_other_project_result.is_ok());
    }

    pub fn should_delete_where_entry_exists<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let entry = Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string(),
            started_at: Utc::now(),
            ended_at: None,
        };
        let all_entries_query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };

        let create_result = repository.create(entry);
        let found_entries_after_create =
            repository.list(all_entries_query.clone(), Paging::default());
        let delete_result = repository.delete(id);
        let found_entries_after_delete =
            repository.list(all_entries_query.clone(), Paging::default());

        assert!(create_result.is_ok());
        assert!(found_entries_after_create.is_ok());
        assert_eq!(found_entries_after_create.unwrap().len(), 1);
        assert!(delete_result.is_ok());
        assert!(found_entries_after_delete.is_ok());
        assert!(found_entries_after_delete.unwrap().is_empty());
    }

    pub fn should_return_err_where_delete_not_exist_entry<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();

        let delete_result = repository.delete(id);

        assert!(delete_result.is_err());
        assert!(
            matches!(delete_result.unwrap_err(), NotExistsById(value) if value == id.to_string())
        );
    }

    pub fn should_return_total_entries<R: EntryRepository>(repository: &R) {
        let all_entry_query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };
        let all_total_before_create = repository.total(all_entry_query.clone());
        let first_project_id = Uuid::now_v7();
        let second_project_id = Uuid::now_v7();
        let first_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: first_project_id,
            name: "first_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });
        let second_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: second_project_id,
            name: "second_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });

        let all_total_after_create = repository.total(all_entry_query.clone());
        let total_filtered_by_first_substr = repository.total(EntryQuery {
            name: EntryNameQuery::Contains("first".to_string()),
            project: EntryProjectQuery::Any,
        });
        let total_filtered_by_second_substr = repository.total(EntryQuery {
            name: EntryNameQuery::Contains("second".to_string()),
            project: EntryProjectQuery::Any,
        });
        let total_filtered_by_first_project = repository.total(EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::In(first_project_id),
        });
        let total_filtered_by_second_project = repository.total(EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::In(second_project_id),
        });

        assert_eq!(all_total_before_create, 0);
        assert!(first_create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert_eq!(all_total_after_create, 2);
        assert_eq!(total_filtered_by_first_substr, 1);
        assert_eq!(total_filtered_by_second_substr, 1);
        assert_eq!(total_filtered_by_first_project, 1);
        assert_eq!(total_filtered_by_second_project, 1);
    }

    pub fn should_return_empty_entries_when_limit_is_zero<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let crete_result = repository.create(Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string().add("_name"),
            started_at: Utc::now(),
            ended_at: None,
        });

        let list_with_not_zero_limit_result = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 1,
            },
        );
        let list_with_zero_limit_result = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 0,
            },
        );

        assert!(crete_result.is_ok());
        assert!(list_with_not_zero_limit_result.is_ok());
        assert!(list_with_not_zero_limit_result.unwrap().len() > 0);
        assert!(list_with_zero_limit_result.is_ok());
        assert!(list_with_zero_limit_result.unwrap().is_empty());
    }

    pub fn should_return_valid_entries_when_limit_is_not_zero<R: EntryRepository>(repository: &R) {
        let id = Uuid::now_v7();
        let crete_result = repository.create(Entry {
            id: id,
            project_id: Uuid::now_v7(),
            name: id.to_string().add("_name"),
            started_at: Utc::now(),
            ended_at: None,
        });

        let list_with_limit_eq_size = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 1,
            },
        );
        let list_with_limit_gt_size = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::Any,
            },
            Paging {
                offset: 0,
                limit: 5,
            },
        );

        assert!(crete_result.is_ok());
        assert!(list_with_limit_eq_size.is_ok());
        assert_eq!(list_with_limit_eq_size.unwrap().len(), 1);
        assert!(list_with_limit_gt_size.is_ok());
        assert_eq!(list_with_limit_gt_size.unwrap().len(), 1);
    }

    pub fn should_can_paginate_by_entries_using_offset_and_limit<R: EntryRepository>(
        repository: &R,
    ) {
        let all_query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };
        let entry_ids: HashSet<Uuid> = (0..5).map(|_| Uuid::now_v7()).collect();
        let entries: Vec<Entry> = entry_ids
            .iter()
            .map(|id| Entry {
                id: id.clone(),
                project_id: Uuid::now_v7(),
                name: id.to_string().add("_name"),
                started_at: Utc::now(),
                ended_at: None,
            })
            .collect();
        let create_entry_result: Vec<super::Result<()>> = entries
            .iter()
            .cloned()
            .map(|entry| repository.create(entry))
            .collect();

        let first_page = repository.list(
            all_query.clone(),
            Paging {
                offset: 0,
                limit: 2,
            },
        );
        let second_page = repository.list(
            all_query.clone(),
            Paging {
                offset: 2,
                limit: 3,
            },
        );
        let third_page = repository.list(
            all_query.clone(),
            Paging {
                offset: 6,
                limit: 5,
            },
        );

        create_entry_result
            .iter()
            .for_each(|result| assert!(result.is_ok()));

        assert!(first_page.is_ok());
        let first_page = first_page.unwrap();
        let first_page_ids: HashSet<Uuid> = first_page.iter().map(|project| project.id).collect();
        assert_eq!(first_page.len(), 2);
        assert_eq!(first_page_ids.len(), first_page.len());
        assert!(entry_ids.is_superset(&first_page_ids));

        assert!(second_page.is_ok());
        let second_page = second_page.unwrap();
        let second_page_ids: HashSet<Uuid> = second_page.iter().map(|project| project.id).collect();
        assert_eq!(second_page.len(), 3);
        assert_eq!(second_page_ids.len(), second_page.len());
        assert!(entry_ids.is_superset(&second_page_ids));
        assert!(second_page_ids.is_disjoint(&first_page_ids));

        assert!(third_page.is_ok());
        assert!(third_page.unwrap().is_empty());

        let mut total_pages = first_page.clone();
        total_pages.append(&mut second_page.clone());
        let mut sorted_pages = total_pages.clone();
        sorted_pages.sort_by(|a, b| a.id.cmp(&b.id));

        let total_pages_ids: Vec<Uuid> = total_pages
            .iter()
            .map(|project| project.id.clone())
            .collect();
        let sorted_pages_ids: Vec<Uuid> = sorted_pages
            .iter()
            .map(|project| project.id.clone())
            .collect();

        assert_eq!(total_pages_ids, sorted_pages_ids);
    }

    pub fn should_can_filter_entries_by_name<R: EntryRepository>(repository: &R) {
        let paging = Paging {
            offset: 0,
            limit: 5,
        };
        let all_entries_query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };
        let list_all_before_create = repository.list(all_entries_query.clone(), paging.clone());

        let first_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            name: "first_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });
        let second_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            name: "second_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });

        let list_all_after_create = repository.list(all_entries_query.clone(), paging.clone());
        let list_filtered_by_first_substr = repository.list(
            EntryQuery {
                name: EntryNameQuery::Contains("first".to_string()),
                project: EntryProjectQuery::Any,
            },
            paging.clone(),
        );
        let list_filtered_by_second_substr = repository.list(
            EntryQuery {
                name: EntryNameQuery::Contains("second".to_string()),
                project: EntryProjectQuery::Any,
            },
            paging.clone(),
        );
        let list_filtered_by_common_infix = repository.list(
            EntryQuery {
                name: EntryNameQuery::Contains("nam".to_string()),
                project: EntryProjectQuery::Any,
            },
            paging.clone(),
        );

        assert!(list_all_before_create.is_ok());
        let list_all_before_create = list_all_before_create.unwrap();
        assert!(list_all_before_create.is_empty());
        assert!(first_create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert!(list_all_after_create.is_ok());
        let list_all_after_create = list_all_after_create.unwrap();
        assert_eq!(list_all_after_create.len(), 2);
        assert!(list_filtered_by_first_substr.is_ok());
        let list_filtered_by_first_substr = list_filtered_by_first_substr.unwrap();
        assert_eq!(list_filtered_by_first_substr.len(), 1);
        assert!(list_filtered_by_second_substr.is_ok());
        let list_filtered_by_second_substr = list_filtered_by_second_substr.unwrap();
        assert_eq!(list_filtered_by_second_substr.len(), 1);
        assert!(list_filtered_by_common_infix.is_ok());
        let list_filtered_by_common_infix = list_filtered_by_common_infix.unwrap();
        assert_eq!(list_filtered_by_common_infix.len(), 2);
    }

    pub fn should_can_filter_entries_by_project_id<R: EntryRepository>(repository: &R) {
        let paging = Paging {
            offset: 0,
            limit: 5,
        };
        let all_entries_query = EntryQuery {
            name: EntryNameQuery::Any,
            project: EntryProjectQuery::Any,
        };
        let list_all_before_create = repository.list(all_entries_query.clone(), paging.clone());
        let first_project_id = Uuid::now_v7();
        let second_project_id = Uuid::now_v7();

        let first_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: first_project_id,
            name: "first_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });
        let second_create_result = repository.create(Entry {
            id: Uuid::now_v7(),
            project_id: second_project_id,
            name: "second_name".to_string(),
            started_at: Utc::now(),
            ended_at: None,
        });

        let list_all_after_create = repository.list(all_entries_query.clone(), paging.clone());
        let list_filtered_by_first_project = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::In(first_project_id),
            },
            paging.clone(),
        );
        let list_filtered_by_second_project = repository.list(
            EntryQuery {
                name: EntryNameQuery::Any,
                project: EntryProjectQuery::In(second_project_id),
            },
            paging.clone(),
        );

        assert!(list_all_before_create.is_ok());
        let list_all_before_create = list_all_before_create.unwrap();
        assert!(list_all_before_create.is_empty());
        assert!(first_create_result.is_ok());
        assert!(second_create_result.is_ok());
        assert!(list_all_after_create.is_ok());
        let list_all_after_create = list_all_after_create.unwrap();
        assert_eq!(list_all_after_create.len(), 2);
        assert!(list_filtered_by_first_project.is_ok());
        let list_filtered_by_first_substr = list_filtered_by_first_project.unwrap();
        assert_eq!(list_filtered_by_first_substr.len(), 1);
        assert!(list_filtered_by_second_project.is_ok());
        let list_filtered_by_second_substr = list_filtered_by_second_project.unwrap();
        assert_eq!(list_filtered_by_second_substr.len(), 1);
    }
}
