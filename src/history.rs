
use std::path;

use gtk;
use rusqlite;

use app;
use storage;
use layout;
use scrolled;
use text;

pub struct Map {
    container: gtk::Box,
    search_entry: gtk::SearchEntry,
    summary: gtk::Label,
    results: gtk::TreeView,
    model: gtk::ListStore,
}

impl Map {

    pub fn new() -> Map {
        Map {
            container: layout::vbox(),
            search_entry: gtk::SearchEntry::new(),
            summary: gtk::Label::new(""),
            results: gtk::TreeView::new(),
            model: gtk::ListStore::new(&[
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
            ]),
        }
    }

    pub fn container(&self) -> &gtk::Box { &self.container }

    pub fn focus(&self) {
        use gtk::prelude::*;

        self.search_entry.grab_focus();
    }
}

fn search(app: &app::Handle, entry: &gtk::SearchEntry) {
    use gtk::prelude::*;

    let text = entry.get_text();
    let stored = app.stored();
    let map = stored.history();
    let text = text.unwrap_or_else(|| String::new());
    let history = app.history();
    let count = history.search(&text, &map.model);
    map.summary.set_text(&format!("{} {}",
        count,
        text::pluralize(count as u64, "result", "results"),
    ));
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;
    use layout::{ BuildBox };
    use pango;

    let map = app.stored();
    let map = map.history();

    let container = &map.container;
    container.add_start(&layout::hbox()
        .add_start(&map.search_entry)
        .add_end(&map.summary)
    );
    container.add_start_fill(&scrolled::create(map.results.clone()));
    container.show_all();

    map.search_entry.connect_search_changed(with_cloned!(app, move |entry| {
        search(&app, entry);
    }));

    map.results.set_model(&map.model);
    search(app, &map.search_entry);

    let title_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_property_ellipsize(pango::EllipsizeMode::End);
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 0);
        column.set_expand(true);
        column
    };

    let uri_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_property_ellipsize(pango::EllipsizeMode::End);
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 1);
        column.set_expand(true);
        column
    };

    let access_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 2);
        column
    };
    
    let view = &map.results;
    view.append_column(&title_column);
    view.append_column(&uri_column);
    view.append_column(&access_column);
    view.set_tooltip_column(1);
    view.set_headers_visible(false);

    view.connect_row_activated(with_cloned!(app, move |view, path, _| {
        load_selected(&app, view, path);
    }));
}

fn load_selected(app: &app::Handle, view: &gtk::TreeView, path: &gtk::TreePath) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let model = view.get_model().expect("model attached to history result view");
    let iter = unwrap_or_return!(model.get_iter(path));
    let uri: String = model.get_value(&iter, 3).get().expect("stored uri in model");

    log_debug!("load from history: {:?}", uri);

    let webview = unwrap_or_return!(app.active_webview());
    webview.load_uri(&uri);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Read,
    ReadWrite,
}

pub struct History {
    storage: Option<storage::Storage>,
    mode: Mode,
}

impl History {

    pub fn open_or_create<P>(path: P, mode: Mode) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {

        let path = path.as_ref();

        Ok(History {
            mode,
            storage: if path.exists() || mode == Mode::ReadWrite {
                Some(storage::Storage::open_or_create(
                    path,
                    |conn| {
                        conn.execute("
                            CREATE VIRTUAL TABLE history USING fts4 (
                                uri TEXT NOT NULL UNIQUE,
                                title TEXT,
                                last_access TEXT NOT NULL
                            )
                        ", &[])?;
                        Ok(())
                    },
                    |_conn| Ok(()),
                )?)
            } else {
                None
            },
        })
    }

    pub fn search(&self, text: &str, model: &gtk::ListStore) -> usize {
        use gtk::prelude::*;

        let text = text.trim();

        let storage = match self.storage.as_ref() {
            Some(storage) => storage,
            None => {
                model.clear();
                return 0;
            },
        };
        storage.with_connection(|conn| {

            let mut cond = String::new();
            let mut params = Vec::new();
            for part in text::parse_search(text) {
                if !cond.is_empty() {
                    cond.push_str(" OR ");
                }
                cond.push_str("uri LIKE ? OR title LIKE ?");
                let term = format!("%{}%", part);
                params.push(term.clone());
                params.push(term);
            }
            let params = params.iter()
                .map(|p| p as &rusqlite::types::ToSql)
                .collect::<Vec<_>>();

            if cond.is_empty() {
                cond.push_str("1=1");
            }

            let mut stmt = conn.prepare(&format!("
                SELECT title, uri, last_access
                FROM history
                WHERE {}
                ORDER BY last_access DESC, title, uri
                LIMIT 100
            ", cond))?;
            let mut rows = stmt.query(&params[..])?;

            model.clear();
            let mut count = 0;
            while let Some(row) = rows.next() {
                count += 1;
                let row = row?;

                let title: Option<String> = row.get(0);
                let title = title.unwrap_or_else(|| String::new());
                let title = text::escape(&title);
                let title: &str = &title;

                let uri: String = row.get(1);
                let raw_uri = &uri;
                let uri = text::escape(&uri);
                let uri: &str = &uri;

                let access: String = row.get(2);
                let access = text::escape(&access);
                let access: &str = &access;

                model.insert_with_values(
                    None,
                    &[0, 1, 2, 3],
                    &[&title, &uri, &access, &raw_uri],
                );
            }
            Ok(count)
        }).expect("history storage search results")
    }

    fn try_write<F>(&self, body: F) -> Result<(), storage::Error>
    where F: FnOnce(&storage::Storage) -> Result<(), storage::Error> {
        if self.mode == Mode::ReadWrite {
            if let Some(ref storage) = self.storage {
                body(storage)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn update_access(&self, uri: &str) -> Result<(), storage::Error> {
        self.try_write(|storage| {
            storage.with_connection(|conn| {
                conn.execute("
                    INSERT OR IGNORE
                    INTO history (uri, last_access)
                    VALUES (?, datetime('now'))
                ", &[&uri])?;
                conn.execute("
                    UPDATE history
                    SET last_access = datetime('now')
                    WHERE uri = ?
                ", &[&uri])?;
                Ok(())
            })
        })
    }

    pub fn update_title(&self, uri: &str, title: &str) -> Result<(), storage::Error> {
        self.try_write(|storage| {
            storage.with_connection(|conn| {
                conn.execute("
                    UPDATE history
                    SET title = ?
                    WHERE uri = ?
                ", &[&title, &uri])?;
                Ok(())
            })
        })
    }
}
