use crate::models::{DateTime, Entry, EntryId};
use crate::repository::EntryNameQuery;
use crate::repository::ProjectNameQuery;
use crate::repository::{
    EntryProjectQuery::{Any, In},
    EntryQuery, Paging, ProjectQuery, Result,
};
use crate::{
    models::{Project, ProjectId},
    repository::{
        BrokenState::{HasDuplicateIds, HasDuplicateNames},
        EntryRepository,
        Error::{self},
        NameAlreadyUsed, ProjectRepository,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::PathBuf, str::FromStr, sync::RwLock};

pub struct JsonProjectRepository {
    path: PathBuf,
    projects: RwLock<Vec<Project>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectsState {
    projects: Vec<ProjectState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectState {
    id: String,
    name: String,
    archived: bool,
}

impl ProjectsState {
    fn from(projects: &[Project]) -> Self {
        let project_states: Vec<ProjectState> = projects
            .iter()
            .cloned()
            .map(|project| ProjectState {
                id: project.id.to_string(),
                name: project.name,
                archived: project.archived,
            })
            .collect();
        Self {
            projects: project_states,
        }
    }
}

impl JsonProjectRepository {
    pub fn new(path: PathBuf) -> Result<Self> {
        let projects = JsonProjectRepository::load(&path)?;
        Ok(Self {
            path: path,
            projects: RwLock::new(projects),
        })
    }

    fn load(path: &PathBuf) -> Result<Vec<Project>> {
        let text = std::fs::read_to_string(path)?;
        let state: ProjectsState = serde_json::from_str(&text)?;
        let mut projects: Vec<Project> = Vec::with_capacity(state.projects.len());
        let mut unique_ids: HashSet<ProjectId> = HashSet::new();
        let mut unique_names: HashSet<String> = HashSet::new();
        for project_state in state.projects.iter().cloned() {
            let id = ProjectId::from_str(project_state.id.as_str())?;
            if !unique_ids.insert(id) {
                return Err(Error::BrokenState(HasDuplicateIds(id.to_string())));
            }
            let name = project_state.name;
            if !unique_names.insert(name.clone()) {
                return Err(Error::BrokenState(HasDuplicateNames(name)));
            }
            projects.push(Project {
                id,
                name,
                archived: project_state.archived,
            });
        }
        Ok(projects)
    }

    fn sync_projects(&self, projects: &[Project]) -> Result<()> {
        let text = serde_json::to_string(&ProjectsState::from(projects))?;
        fs::write(&self.path, text)?;
        Ok(())
    }

    pub(crate) fn new_with_ensure_file(path: PathBuf) -> Result<Self> {
        if path.exists() {
            return Self::new(path);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, r#"{"projects":[]}"#).unwrap();

        Self::new(path)
    }
}

impl ProjectQuery {
    fn is_match(&self, project: &Project) -> bool {
        match &self.name {
            ProjectNameQuery::Contains(name) => project.name.contains(name.as_str()),
            ProjectNameQuery::Any => true,
        }
    }
}

impl ProjectRepository for JsonProjectRepository {
    fn total(&self, query: ProjectQuery) -> i64 {
        self.projects
            .read()
            .unwrap()
            .iter()
            .filter(|project| query.is_match(project))
            .count() as i64
    }

    fn list(&self, query: ProjectQuery, paging: Paging) -> Result<Vec<Project>> {
        if paging.limit == 0 {
            return Ok(Vec::new());
        }
        let mut projects: Vec<Project> = self
            .projects
            .read()
            .unwrap()
            .iter()
            .filter(|project| query.is_match(project))
            .cloned()
            .collect();
        projects.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(projects
            .iter()
            .skip(paging.offset)
            .take(paging.limit)
            .cloned()
            .collect())
    }

    fn create(&self, create_project: crate::models::Project) -> Result<()> {
        let mut projects = self.projects.write().unwrap();
        let exists_project = projects
            .iter()
            .find(|project| create_project.id == project.id || create_project.name == project.name)
            .cloned();
        if let Some(exists_project) = exists_project {
            if exists_project.id == create_project.id {
                return Err(Error::IdAlreadyUsed(exists_project.id.to_string()));
            }
            return Err(Error::NameAlreadyUsed(NameAlreadyUsed {
                name: exists_project.name,
                id: exists_project.id.to_string(),
            }));
        }
        projects.push(create_project);
        self.sync_projects(&projects)?;
        Ok(())
    }

    fn update(&self, update_project: crate::models::Project) -> Result<()> {
        let mut projects = self.projects.write().unwrap();
        let mut exists_projects: Vec<&mut Project> = projects
            .iter_mut()
            .filter(|project| {
                update_project.id == project.id || update_project.name == project.name
            })
            .collect();
        assert!(exists_projects.len() < 3);
        if exists_projects.len() == 0 {
            return Err(Error::NotExistsById(update_project.id.to_string()));
        }
        if exists_projects.len() == 2 {
            let project_with_same_name = if exists_projects[0].id == update_project.id {
                &exists_projects[1]
            } else {
                &exists_projects[0]
            };
            return Err(Error::NameAlreadyUsed(NameAlreadyUsed {
                name: project_with_same_name.name.clone(),
                id: project_with_same_name.id.to_string(),
            }));
        }
        *exists_projects[0] = update_project;
        self.sync_projects(&projects)?;
        Ok(())
    }

    fn delete(&self, id: crate::models::ProjectId) -> Result<()> {
        let mut projects = self.projects.write().unwrap();
        let removed_count = projects.extract_if(.., |project| project.id == id).count();
        if removed_count == 0 {
            return Err(Error::NotExistsById(id.to_string()));
        }
        self.sync_projects(&projects)?;
        Ok(())
    }
}

pub struct JsonEntryRepository {
    path: PathBuf,
    entries: RwLock<Vec<Entry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntriesState {
    entries: Vec<EntryState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryState {
    id: String,
    project_id: String,
    name: String,
    started_at: DateTime,
    ended_at: Option<DateTime>,
}

impl EntriesState {
    fn from(entries: &[Entry]) -> Self {
        let entry_states: Vec<EntryState> = entries
            .iter()
            .cloned()
            .map(|entry| EntryState {
                id: entry.id.to_string(),
                project_id: entry.project_id.to_string(),
                name: entry.name,
                started_at: entry.started_at,
                ended_at: entry.ended_at,
            })
            .collect();
        Self {
            entries: entry_states,
        }
    }
}

impl JsonEntryRepository {
    pub fn new(path: PathBuf) -> Result<Self> {
        let entries = JsonEntryRepository::load(&path)?;
        Ok(Self {
            path: path,
            entries: RwLock::new(entries),
        })
    }

    fn load(path: &PathBuf) -> Result<Vec<Entry>> {
        let text = std::fs::read_to_string(path)?;
        let state: EntriesState = serde_json::from_str(&text)?;
        let mut entries: Vec<Entry> = Vec::with_capacity(state.entries.len());
        let mut unique_ids: HashSet<ProjectId> = HashSet::with_capacity(state.entries.len());
        let mut unique_names: HashSet<String> = HashSet::with_capacity(state.entries.len());
        for entry_state in state.entries.iter().cloned() {
            let id = EntryId::from_str(entry_state.id.as_str())?;
            let project_id = ProjectId::from_str(entry_state.project_id.as_str())?;
            if !unique_ids.insert(id) {
                return Err(Error::BrokenState(HasDuplicateIds(id.to_string())));
            }
            let name = entry_state.name;
            if !unique_names.insert(name.clone()) {
                return Err(Error::BrokenState(HasDuplicateNames(name)));
            }
            entries.push(Entry {
                id,
                name,
                project_id: project_id,
                started_at: entry_state.started_at,
                ended_at: entry_state.ended_at,
            });
        }
        Ok(entries)
    }

    fn sync_entries(&self, entries: &[Entry]) -> Result<()> {
        let text = serde_json::to_string(&EntriesState::from(entries))?;
        fs::write(&self.path, text)?;
        Ok(())
    }

    pub(crate) fn new_with_ensure_file(path: PathBuf) -> Result<Self> {
        if path.exists() {
            return Self::new(path);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, r#"{"entries":[]}"#).unwrap();

        Self::new(path)
    }
}

impl EntryQuery {
    fn is_match(&self, entry: &Entry) -> bool {
        let name_is_match = match &self.name {
            EntryNameQuery::Contains(query_name) => entry.name.contains(query_name.as_str()),
            EntryNameQuery::Any => true,
        };
        if !name_is_match {
            return false;
        }
        let project_is_match = match &self.project {
            In(query_project_id) => entry.project_id == *query_project_id,
            Any => true,
        };
        project_is_match
    }
}

impl EntryRepository for JsonEntryRepository {
    fn total(&self, query: EntryQuery) -> i64 {
        self.entries
            .read()
            .unwrap()
            .iter()
            .filter(|entry| query.is_match(entry))
            .count() as i64
    }

    fn list(&self, query: EntryQuery, paging: Paging) -> Result<Vec<Entry>> {
        if paging.limit == 0 {
            return Ok(Vec::new());
        }
        let mut entries: Vec<Entry> = self
            .entries
            .read()
            .unwrap()
            .iter()
            .filter(|entry| query.is_match(entry))
            .cloned()
            .collect();
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(entries
            .iter()
            .skip(paging.offset)
            .take(paging.limit)
            .cloned()
            .collect())
    }

    fn create(&self, create_entry: Entry) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        let exist_entry = entries
            .iter()
            .find(|entry| {
                entry.id == create_entry.id
                    || (entry.name == create_entry.name
                        && entry.project_id == create_entry.project_id)
            })
            .cloned();
        if let Some(exist_entry) = exist_entry {
            if exist_entry.id == create_entry.id {
                return Err(Error::IdAlreadyUsed(exist_entry.id.to_string()));
            }
            return Err(Error::NameAlreadyUsed(NameAlreadyUsed {
                name: exist_entry.name,
                id: exist_entry.id.to_string(),
            }));
        }
        entries.push(create_entry);
        self.sync_entries(&entries)?;
        Ok(())
    }

    fn update(&self, update_entry: Entry) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        let already_used_name_in_same_project = entries
            .iter()
            .find(|entry| {
                entry.id != update_entry.id
                    && entry.project_id == update_entry.project_id
                    && entry.name == update_entry.name
            })
            .cloned();
        if let Some(already_used_name_in_same_project) = already_used_name_in_same_project {
            return Err(Error::NameAlreadyUsed(NameAlreadyUsed {
                name: already_used_name_in_same_project.name,
                id: already_used_name_in_same_project.id.to_string(),
            }));
        }
        let exist_entry = entries.iter_mut().find(|entry| entry.id == update_entry.id);
        let Some(exist_entry) = exist_entry else {
            return Err(Error::NotExistsById(update_entry.id.to_string()));
        };
        *exist_entry = update_entry;
        self.sync_entries(&entries)?;
        Ok(())
    }

    fn delete(&self, id: EntryId) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        let removed_count = entries.extract_if(.., |entry| entry.id == id).count();
        if removed_count == 0 {
            return Err(Error::NotExistsById(id.to_string()));
        }
        self.sync_entries(&entries)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::NamedTempFile;

    use super::super::contract;
    use crate::repository::json::{JsonEntryRepository, JsonProjectRepository};

    #[test]
    fn should_sucess_create_project_when_model_is_valid() {
        let fixture = create_project_repository_fixture();
        contract::should_sucess_create_project_when_model_is_valid(&fixture.repository())
    }

    #[test]
    fn should_return_err_when_create_project_with_duplicate_id() {
        let fixture = create_project_repository_fixture();
        contract::should_return_err_when_create_project_with_duplicate_id(&fixture.repository())
    }

    #[test]
    fn should_return_err_when_create_project_with_duplicate_name() {
        let fixture = create_project_repository_fixture();
        contract::should_return_err_when_create_project_with_duplicate_name(&fixture.repository())
    }

    #[test]
    fn should_update_where_project_exist() {
        let fixture = create_project_repository_fixture();
        contract::should_update_where_project_exist(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_update_not_exist_project() {
        let fixture = create_project_repository_fixture();
        contract::should_return_err_when_update_not_exist_project(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_update_project_with_already_used_name() {
        let fixture = create_project_repository_fixture();
        contract::should_return_err_when_update_project_with_already_used_name(
            &fixture.repository(),
        );
    }

    #[test]
    fn should_delete_where_project_exists() {
        let fixture = create_project_repository_fixture();
        contract::should_delete_where_project_exists(&fixture.repository());
    }

    #[test]
    fn should_return_err_where_delete_not_exist_project() {
        let fixture = create_project_repository_fixture();
        contract::should_return_err_where_delete_not_exist_project(&fixture.repository());
    }

    #[test]
    fn should_return_total_projects() {
        let fixture = create_project_repository_fixture();
        contract::should_return_total_projects(&fixture.repository());
    }

    #[test]
    fn should_return_empty_projects_when_limit_is_zero() {
        let fixture = create_project_repository_fixture();
        contract::should_return_empty_projects_when_limit_is_zero(&fixture.repository());
    }

    #[test]
    fn should_return_valid_projects_when_limit_is_not_zero() {
        let fixture = create_project_repository_fixture();
        contract::should_return_valid_projects_when_limit_is_not_zero(&fixture.repository());
    }

    #[test]
    fn should_can_paginate_by_projects_using_offset_and_limit() {
        let fixture = create_project_repository_fixture();
        contract::should_can_paginate_by_projects_using_offset_and_limit(&fixture.repository());
    }

    #[test]
    fn should_can_filter_projects_by_name() {
        let fixture = create_project_repository_fixture();
        contract::should_can_filter_projects_by_name(&fixture.repository());
    }

    #[test]
    fn should_sucess_create_entry_when_model_is_valid() {
        let fixture = create_entry_repository_fixture();
        contract::should_sucess_create_entry_when_model_is_valid(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_create_entry_with_duplicate_id() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_err_when_create_entry_with_duplicate_id(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_create_project_with_duplicate_name_in_same_project() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_err_when_create_project_with_duplicate_name_in_same_project(
            &fixture.repository(),
        );
    }

    #[test]
    fn should_update_where_entry_exist() {
        let fixture = create_entry_repository_fixture();
        contract::should_update_where_entry_exist(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_update_not_exist_entry() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_err_when_update_not_exist_entry(&fixture.repository());
    }

    #[test]
    fn should_return_err_when_update_entry_with_already_used_name_in_same_project() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_err_when_update_entry_with_already_used_name_in_same_project(
            &fixture.repository(),
        );
    }

    #[test]
    fn should_delete_where_entry_exists() {
        let fixture = create_entry_repository_fixture();
        contract::should_delete_where_entry_exists(&fixture.repository());
    }

    #[test]
    fn should_return_err_where_delete_not_exist_entry() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_err_where_delete_not_exist_entry(&fixture.repository());
    }

    #[test]
    fn should_return_total_entries() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_total_entries(&fixture.repository());
    }

    #[test]
    fn should_return_empty_entries_when_limit_is_zero() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_empty_entries_when_limit_is_zero(&fixture.repository());
    }

    #[test]
    fn should_return_valid_entries_when_limit_is_not_zero() {
        let fixture = create_entry_repository_fixture();
        contract::should_return_valid_entries_when_limit_is_not_zero(&fixture.repository());
    }

    #[test]
    fn should_can_paginate_by_entries_using_offset_and_limit() {
        let fixture = create_entry_repository_fixture();
        contract::should_can_paginate_by_entries_using_offset_and_limit(&fixture.repository());
    }

    #[test]
    fn should_can_filter_entries_by_name() {
        let fixture = create_entry_repository_fixture();
        contract::should_can_filter_entries_by_name(&fixture.repository());
    }

    #[test]
    fn should_can_filter_entries_by_project_id() {
        let fixture = create_entry_repository_fixture();
        contract::should_can_filter_entries_by_project_id(&fixture.repository());
    }

    struct RepositoryFixture<R, F>
    where
        F: Fn(&Path) -> R,
    {
        file: NamedTempFile,
        repository_factory: F,
    }

    impl<R, F> RepositoryFixture<R, F>
    where
        F: Fn(&Path) -> R,
    {
        fn repository(&self) -> R {
            (self.repository_factory)(self.file.path())
        }
    }

    fn create_project_repository_fixture()
    -> RepositoryFixture<JsonProjectRepository, impl Fn(&Path) -> JsonProjectRepository> {
        RepositoryFixture {
            file: empty_projects_file(),
            repository_factory: |file_path| {
                JsonProjectRepository::new(file_path.to_path_buf()).unwrap()
            },
        }
    }

    fn create_entry_repository_fixture()
    -> RepositoryFixture<JsonEntryRepository, impl Fn(&Path) -> JsonEntryRepository> {
        RepositoryFixture {
            file: empty_entries_file(),
            repository_factory: |file_path| {
                JsonEntryRepository::new(file_path.to_path_buf()).unwrap()
            },
        }
    }

    fn empty_projects_file() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), r#"{"projects":[]}"#).unwrap();
        file
    }

    fn empty_entries_file() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), r#"{"entries":[]}"#).unwrap();
        file
    }
}
