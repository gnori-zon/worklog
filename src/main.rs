mod models;
mod repository;
mod view;
mod view_model;

use crate::repository::EntryRepository;
use crate::repository::ProjectRepository;
use crate::repository::ProjectStatsQuery;
use crate::repository::ProjectStatsRepository;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> color_eyre::Result<()> {
    let mut terminal: DefaultTerminal = ratatui::init();
    let mut app = new_app().unwrap();
    while !app.model.should_quit {
        terminal.draw(|frame| app.render(frame))?;
        app.handle_event(event::read()?);
        if app.model.should_quit {
            break;
        }
    }
    ratatui::restore();
    Ok(())
}

struct App<S: Service> {
    model: view_model::AppModel,
    service: S,
}

fn new_app() -> Result<App<JsonServiceImpl>, repository::Error> {
    let data_dir = PathBuf::from("./.data");
    let service = JsonServiceImpl {
        project_repository: repository::json::JsonProjectRepository::new_with_ensure_file(
            data_dir.join("projects.json"),
        )?,
        entry_repository: repository::json::JsonEntryRepository::new_with_ensure_file(
            data_dir.join("entries.json"),
        )?,
    };
    let default_paging = repository::Paging {
        offset: 0,
        limit: 100_000,
    };
    let projects = service
        .project_repository()
        .list(repository::ProjectQuery::default(), default_paging.clone())
        .unwrap();
    let app = App {
        model: view_model::AppModel {
            should_quit: false,
            version: "0.1.0".into(),
            focus: view_model::Focus::Projects,
            projects: view_model::projects::ProjectsModel {
                values: projects.clone(),
                stats: service
                    .project_stats_repository()
                    .list(ProjectStatsQuery::WeeklyBy(
                        projects.iter().map(|project| project.id).collect(),
                    ))
                    .unwrap()
                    .iter()
                    .fold(HashMap::new(), |mut acc, project_stat| {
                        acc.insert(project_stat.project_id, project_stat.clone());
                        acc
                    }),
                query: repository::ProjectQuery::default(),
                paging: default_paging.clone(),
                cursor: view_model::Cursor::default(),
                mode: view_model::projects::ProjectModelMode::View(None),
            },
            entries: view_model::entries::EntriesModel {
                values: service
                    .entry_repository()
                    .list(repository::EntryQuery::default(), default_paging.clone())
                    .unwrap(),
                paging: default_paging.clone(),
                cursor: view_model::Cursor::default(),
                query: repository::EntryQuery {
                    name: repository::EntryNameQuery::default(),
                    project: repository::EntryProjectQuery::default(),
                    date: repository::EntryDateQuery::Running,
                },
                mode: view_model::entries::EntriesModelMode::View(None),
            },
        },
        service: service,
    };
    Ok(app)
}

impl<S: Service> App<S> {
    fn render(&self, frame: &mut Frame) {
        view::render_app(frame, &self.model);
    }

    fn handle_event(&mut self, event: Event) {
        let mut msg = view::input_mapper::map_input(&self.model, event);
        while msg.is_some() {
            msg = self.model.handle_msg(msg.unwrap(), &self.service);
        }
    }
}

trait Service {
    fn project_repository(&self) -> &impl repository::ProjectRepository;
    fn entry_repository(&self) -> &impl repository::EntryRepository;
    fn project_stats_repository(&self) -> &impl repository::ProjectStatsRepository;
}

struct JsonServiceImpl {
    project_repository: repository::json::JsonProjectRepository,
    entry_repository: repository::json::JsonEntryRepository,
}

impl Service for JsonServiceImpl {
    fn project_repository(&self) -> &impl repository::ProjectRepository {
        &self.project_repository
    }

    fn entry_repository(&self) -> &impl repository::EntryRepository {
        &self.entry_repository
    }

    fn project_stats_repository(&self) -> &impl repository::ProjectStatsRepository {
        &self.entry_repository
    }
}
