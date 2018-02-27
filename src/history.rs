
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
        use gtk::{ WidgetExt };

        self.search_entry.grab_focus();
    }
}

fn search(app: &app::Handle, entry: &gtk::SearchEntry) {
    use gtk::{ EntryExt, LabelExt };

    let text = entry.get_text();
    let stored = try_extract!(app.stored());
    let map = stored.history();
    let text = text.unwrap_or_else(|| String::new());
    let history = try_extract!(app.history());
    let count = history.search(&text, &map.model);
    map.summary.set_text(&format!("{} {}",
        count,
        text::pluralize(count as u64, "result", "results"),
    ));
}

pub fn setup(app: &app::Handle) {
    use gtk::{
        WidgetExt, SearchEntryExt, TreeViewExt, CellRendererTextExt, TreeViewColumnExt,
        CellLayoutExt,
    };
    use layout::{ BuildBox };
    use pango;

    let map = try_extract!(app.stored());
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
        let title_column = gtk::TreeViewColumn::new();
        let title_cell = gtk::CellRendererText::new();
        title_cell.set_property_ellipsize(pango::EllipsizeMode::End);
        title_column.pack_start(&title_cell, true);
        title_column.add_attribute(&title_cell, "text", 0);
        title_column.set_expand(true);
        title_column.set_title("Page Title");
        title_column
    };

    let uri_column = {
        let uri_column = gtk::TreeViewColumn::new();
        let uri_cell = gtk::CellRendererText::new();
        uri_cell.set_property_ellipsize(pango::EllipsizeMode::End);
        uri_column.pack_start(&uri_cell, true);
        uri_column.add_attribute(&uri_cell, "text", 1);
        uri_column.set_expand(true);
        uri_column.set_title("Resource");
        uri_column
    };

    let access_column = {
        let access_column = gtk::TreeViewColumn::new();
        let access_cell = gtk::CellRendererText::new();
        access_column.pack_start(&access_cell, true);
        access_column.add_attribute(&access_cell, "text", 2);
        access_column.set_title("Last Access");
        access_column
    };
    
    let view = &map.results;
    view.append_column(&title_column);
    view.append_column(&uri_column);
    view.append_column(&access_column);
    view.set_tooltip_column(1);
    view.set_headers_visible(true);

    view.connect_row_activated(with_cloned!(app, move |view, path, _| {
        use gtk::{ TreeModelExt };
        use webkit2gtk::{ WebViewExt };

        let model = try_extract!(view.get_model());
        let iter = try_extract!(model.get_iter(path));
        let uri: String = model.get_value(&iter, 3).get().expect("stored uri in model");

        log_debug!("load from history: {:?}", uri);

        let webview = try_extract!(app.active_webview());
        webview.load_uri(&uri);
    }));
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
        use gtk::{ ListStoreExt, ListStoreExtManual };

        let text = text.trim();

        let storage = try_extract!(self.storage.as_ref());
        storage.with_connection(|conn| {

            let mut cond = String::new();
            let mut params = Vec::new();
            for part in text.split_whitespace() {
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
        }).expect("history search successful")
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
