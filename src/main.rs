mod models;
mod repository;
mod view;
mod view_model;

use crate::repository::EntryRepository;
use crate::repository::ProjectRepository;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
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
    layout: view::AppLayout,
}

fn new_app() -> Result<App<JsonServiceImpl>, repository::Error> {
    let data_dir = PathBuf::from("./.data");
    let projects_repository = repository::json::JsonProjectRepository::new_with_ensure_file(
        data_dir.join("projects.json"),
    )?;
    let entries_repository =
        repository::json::JsonEntryRepository::new_with_ensure_file(data_dir.join("entries.json"))?;
    let app = App {
        model: view_model::AppModel {
            should_quit: false,
            version: "0.1.0".into(),
            focus: view_model::Focus::ProjectsView,
            projects: view_model::projects::ProjectsModel {
                values: projects_repository
                    .list(
                        repository::ProjectQuery::default(),
                        repository::Paging::default(),
                    )
                    .unwrap(),
                query: repository::ProjectQuery::default(),
                paging: repository::Paging::default(),
                cursor: view_model::Cursor::default(),
                mode: view_model::projects::ProjectModelMode::View(None),
            },
            entries: view_model::entries::EntriesModel {
                values: entries_repository
                    .list(
                        repository::EntryQuery::default(),
                        repository::Paging::default(),
                    )
                    .unwrap(),
                paging: repository::Paging::default(),
                cursor: view_model::Cursor::default(),
                query: repository::EntryQuery::default(),
                mode: view_model::entries::EntriesModelMode::View(None),
            },
        },
        service: JsonServiceImpl {
            project_repository: projects_repository,
            entry_repository: entries_repository,
        },
        layout: view::AppLayout::new(),
    };
    Ok(app)
}

impl<S: Service> App<S> {
    fn render(&self, frame: &mut Frame) {
        self.layout.render(frame, &self.model);
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
}
