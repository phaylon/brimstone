
use std::path;
use std::cell;
use std::rc;

use gtk;
use rusqlite;

use app;
use storage;
use layout;
use window;
use text;

const RES_OK: i32 = 2;
const RES_CANCEL: i32 = 3;

pub struct Map {
    container: gtk::Box,
    list: gtk::TreeView,
    model: gtk::ListStore,
    add_button: gtk::Button,
    remove_button: gtk::Button,
    edit_button: gtk::Button,
    add_dialog: Dialog,
    edit_dialog: Dialog,
}

impl Map {

    pub fn new() -> Map {
        let icon_size = gtk::IconSize::Button.into();
        Map {
            container: layout::vbox(),
            list: gtk::TreeView::new(),
            add_button: gtk::Button::new_from_icon_name("gtk-add", icon_size),
            remove_button: gtk::Button::new_from_icon_name("gtk-remove", icon_size),
            edit_button: gtk::Button::new_from_icon_name("gtk-edit", icon_size),
            add_dialog: Dialog::new("Add Shortcut"),
            edit_dialog: Dialog::new("Edit Shortcut"),
            model: gtk::ListStore::new(&[
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
            ]),
        }
    }

    pub fn container(&self) -> &gtk::Box { &self.container }

    pub fn focus(&self) {}
}

struct Dialog {
    dialog: gtk::Dialog,
    ok_button: gtk::Widget,
    name_entry: gtk::Entry,
    uri_entry: gtk::Entry,
    existing: rc::Rc<cell::RefCell<Vec<String>>>,
}

impl Dialog {

    fn new(title: &str) -> Dialog {
        use gtk::prelude::*;

        let dialog = gtk::Dialog::new();
        dialog.set_title(title);
        let ok_button = dialog.add_button("Ok", RES_OK);
        dialog.add_button("Cancel", RES_CANCEL);
        dialog.set_default_response(RES_OK);
        dialog.set_modal(true);
        dialog.set_destroy_with_parent(true);

        let name_entry = gtk::Entry::new();
        let uri_entry = gtk::Entry::new();

        let grid = gtk::Grid::new();
        grid.attach(&gtk::Label::new("Name"), 0, 0, 1, 1);
        grid.attach(&name_entry, 1, 0, 1, 1);
        grid.attach(&gtk::Label::new("URI"), 0, 1, 1, 1);
        grid.attach(&uri_entry, 1, 1, 1, 1);
        grid.show_all();

        dialog.get_content_area().add(&grid);
        
        let existing = rc::Rc::new(cell::RefCell::new(Vec::new()));

        name_entry.connect_property_text_notify({
            let ok_button = ok_button.clone();
            let existing = existing.clone();
            move |entry| {
                let existing = existing.borrow();
                let is_valid = entry
                    .get_text()
                    .map(|text| name_is_valid(&existing, &text))
                    .unwrap_or(false);
                ok_button.set_sensitive(is_valid);
            }
        });

        Dialog {
            dialog,
            ok_button,
            name_entry,
            uri_entry,
            existing,
        }
    }

    fn set(&self, existing: Vec<String>, name: Option<&str>, uri: Option<&str>) {
        use gtk::prelude::*;

        let is_valid = name.map(|text| name_is_valid(&existing, text)).unwrap_or(false);
        *self.existing.borrow_mut() = existing;
        self.ok_button.set_sensitive(is_valid);
        self.name_entry.set_text(name.unwrap_or(""));
        self.uri_entry.set_text(uri.unwrap_or(""));
    }

    fn get(&self) -> (String, String) {
        use gtk::prelude::*;

        (   self.name_entry.get_text().unwrap_or_else(|| String::new()),
            self.uri_entry.get_text().unwrap_or_else(|| String::new()),
        )
    }
}

fn name_is_valid(existing: &[String], name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '-' | '_' => true,
            _ => false,
        })
        && !existing.contains(&name.into())
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;
    use layout::{ BuildBox };
    use scrolled;
    use pango;

    let window = app.window();
    let shortcuts = app.shortcuts();

    let map = app.stored();
    let map = map.shortcuts();
    map.container().add_start_fill(&scrolled::create(map.list.clone()));
    map.container().add_start(&layout::hbox()
        .add_start(&map.add_button)
        .add_start(&map.edit_button)
        .add_start(&map.remove_button)
    );
    map.list.set_model(&map.model);

    let name_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 0);
        column.set_sort_column_id(0);
        column.set_expand(false);
        column
    };

    let uri_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_property_ellipsize(pango::EllipsizeMode::End);
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", 1);
        column.set_sort_column_id(1);
        column.set_expand(true);
        column
    };
    
    map.list.append_column(&name_column);
    map.list.append_column(&uri_column);
    map.list.set_tooltip_column(1);
    map.list.set_headers_visible(false);
    map.list.get_selection().set_mode(gtk::SelectionMode::Single);

    map.edit_button.set_sensitive(false);
    map.remove_button.set_sensitive(false);

    map.model.set_sort_column_id(gtk::SortColumn::Index(0), gtk::SortType::Ascending);

    map.add_dialog.dialog.set_transient_for(&window);
    map.edit_dialog.dialog.set_transient_for(&window);

    shortcuts.populate(&map.model);

    map.list.get_selection().connect_changed(with_cloned!(app, move |selection| {
        on_selection_change(&app, selection);
    }));

    map.edit_button.connect_clicked(with_cloned!(app, move |_button| {
        edit_selected_shortcut(&app);
    }));

    map.add_button.connect_clicked(with_cloned!(app, move |_button| {
        add_new_shortcut(&app);
    }));

    map.remove_button.connect_clicked(with_cloned!(app, move |_button| {
        remove_selected_shortcut(&app);
    }));
}

fn on_selection_change(app: &app::Handle, selection: &gtk::TreeSelection) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.shortcuts();
    if let Some(_) = selection.get_selected() {
        map.edit_button.set_sensitive(true);
        map.remove_button.set_sensitive(true);
    } else {
        map.edit_button.set_sensitive(false);
        map.remove_button.set_sensitive(false);
    }
}

fn edit_selected_shortcut(app: &app::Handle) {
    use gtk::prelude::*;

    let shortcuts = app.shortcuts();
    let map = app.stored();
    let map = map.shortcuts();
    let (model, iter) = unwrap_or_return!(map.list.get_selection().get_selected());
    let name: String = model.get_value(&iter, 0).get().expect("selected value in model");
    let uri = shortcuts.get(&name).expect("stored uri value");
    let mut names = shortcuts.names();
    names.retain(|name| &name != &name);

    map.edit_dialog.set(names, Some(&name), Some(&uri));
    let result = map.edit_dialog.dialog.run();
    map.edit_dialog.dialog.hide();

    if result == RES_OK {
        shortcuts.remove(&name);
        let (new_name, new_uri) = map.edit_dialog.get();
        shortcuts.add(&new_name, &new_uri);
        shortcuts.populate(&map.model);
    }
}

fn add_new_shortcut(app: &app::Handle) {
    use gtk::prelude::*;

    let shortcuts = app.shortcuts();
    let map = app.stored();
    let map = map.shortcuts();
    map.add_dialog.set(shortcuts.names(), None, None);
    let result = map.add_dialog.dialog.run();
    map.add_dialog.dialog.hide();
    if result == RES_OK {
        let (name, uri) = map.add_dialog.get();
        shortcuts.add(&name, &uri);
        shortcuts.populate(&map.model);
    }
}

fn remove_selected_shortcut(app: &app::Handle) {
    use gtk::prelude::*;

    let shortcuts = app.shortcuts();
    let window = app.window();
    let map = app.stored();
    let map = map.shortcuts();
    let (model, iter) = unwrap_or_return!(map.list.get_selection().get_selected());
    let name: String = model.get_value(&iter, 0).get().expect("selected value in store");
    let result = window::confirm_action(
        &window,
        &format!("Really remove shortcut '{}'?", name),
        &[("Ok", RES_OK), ("Cancel", RES_CANCEL)],
        RES_OK,
    );
    if result == RES_OK {
        shortcuts.remove(&name);
        shortcuts.populate(&map.model);
    }
}

pub struct Shortcuts {
    storage: storage::Storage,
    items: cell::RefCell<Vec<(String, String)>>,
}

impl Shortcuts {

    pub fn open_or_create<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {

        let storage = storage::Storage::open_or_create(
            path,
            init_storage,
            storage::do_nothing,
        )?;

        let items = storage.with_connection(|conn| {
            let mut stmt = conn.prepare("SELECT name, uri FROM shortcuts")?;
            let mut rows = stmt.query(&[])?;
            let mut items = Vec::new();
            while let Some(row) = rows.next() {
                let row = row?;
                let uri: String = row.get(1);
                let uri = text::escape(&uri);
                items.push((row.get(0), uri.into()));
            }
            Ok(cell::RefCell::new(items))
        })?;

        Ok(Shortcuts {
            storage,
            items,
        })
    }

    pub fn names(&self) -> Vec<String> {
        self.items.borrow().iter().map(|item| item.0.clone()).collect()
    }

    pub fn has(&self, name: &str) -> bool {
        self.items.borrow().iter().any(|item| &item.0 == name)
    }

    pub fn get(&self, name: &str) -> Option<String> {
        for item in self.items.borrow().iter() {
            if &item.0 == name {
                return Some(item.1.clone());
            }
        }
        None
    }

    pub fn add(&self, name: &str, uri: &str) {
        self.storage.with_transaction(|tx| {
            tx.execute("INSERT INTO shortcuts (name, uri) VALUES (?, ?)", &[&name, &uri])?;
            self.items.borrow_mut().push((name.into(), uri.into()));
            Ok(())
        }).expect("shortcut storage addition")
    }

    pub fn remove(&self, name: &str) {
        self.storage.with_transaction(|tx| {
            tx.execute("DELETE FROM shortcuts WHERE name = ?", &[&name])?;
            self.items.borrow_mut().retain(|item| &item.0 != name);
            Ok(())
        }).expect("shortcut storage removal")
    }

    pub fn populate(&self, model: &gtk::ListStore) {
        use gtk::prelude::*;

        model.clear();
        for (name, uri) in self.items.borrow().clone() {
            let uri = text::escape(&uri);
            let uri: &str = &uri;
            model.insert_with_values(None, &[0, 1], &[&name, &uri]);
        }
    }
}

fn init_storage(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("
        CREATE TABLE shortcuts (
            name TEXT UNIQUE NOT NULL,
            uri TEXT NOT NULL
        )
    ", &[])?;
    let presets = &[
        ("g", "https://www.google.com/search?q=%s"),
        ("r", "https://www.reddit.com/r/%s"),
    ];
    for &(name, uri) in presets {
        conn.execute("INSERT INTO shortcuts (name, uri) VALUES (?, ?)", &[&name, &uri])?;
    }
    Ok(())
}
