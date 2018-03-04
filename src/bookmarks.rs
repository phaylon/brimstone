
use std::path;
use std::collections;

use gtk;
use rusqlite;

use app;
use layout;
use storage;
use scrolled;
use text;
use window;

type TagKind = i32;
type BookmarkId = i64;

const TAG_LIST_TAG: i32 = 0;
const TAG_LIST_ALL: i32 = 1;
const TAG_LIST_UNTAGGED: i32 = 2;

const RES_OK: i32 = 2;
const RES_CANCEL: i32 = 3;

const TAG_COL_NAME: u32 = 0;
const TAG_COL_COUNT: u32 = 1;
const TAG_COL_NAME_RAW: u32 = 2;
const TAG_COL_KIND: u32 = 3;

const RESULT_COL_TITLE: u32 = 0;
const RESULT_COL_URI: u32 = 1;
const RESULT_COL_ID: u32 = 2;
const RESULT_COL_VISIBLE: u32 = 3;

pub struct Map {
    container: gtk::Box,
    popover: gtk::Popover,
    save_current_button: gtk::Button,
    remove_current_button: gtk::Button,
    title_entry: gtk::Entry,
    tags_entry: gtk::Entry,
    tag_list: gtk::TreeView,
    tag_list_model: gtk::ListStore,
    result_list: gtk::TreeView,
    result_list_model: gtk::ListStore,
    search_entry: gtk::SearchEntry,
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
            popover: gtk::Popover::new::<gtk::Box, _>(None),
            save_current_button: gtk::Button::new_with_label("Save"),
            remove_current_button: gtk::Button::new_with_label("Remove"),
            title_entry: gtk::Entry::new(),
            tags_entry: gtk::Entry::new(),
            search_entry: gtk::SearchEntry::new(),
            add_button: gtk::Button::new_from_icon_name("gtk-add", icon_size),
            remove_button: gtk::Button::new_from_icon_name("gtk-remove", icon_size),
            edit_button: gtk::Button::new_from_icon_name("gtk-edit", icon_size),
            add_dialog: Dialog::new("Add Bookmark"),
            edit_dialog: Dialog::new("Edit Bookmark"),
            tag_list: gtk::TreeView::new(),
            tag_list_model: gtk::ListStore::new(&[
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
                <i32 as gtk::StaticType>::static_type(),
            ]),
            result_list: gtk::TreeView::new(),
            result_list_model: gtk::ListStore::new(&[
                <String as gtk::StaticType>::static_type(),
                <String as gtk::StaticType>::static_type(),
                <BookmarkId as gtk::StaticType>::static_type(),
                <bool as gtk::StaticType>::static_type(),
            ]),
        }
    }

    pub fn container(&self) -> &gtk::Box { &self.container }

    pub fn focus(&self) {
        use gtk::prelude::*;

        self.search_entry.grab_focus();
    }
}

struct Dialog {
    dialog: gtk::Dialog,
    ok_button: gtk::Widget,
    title_entry: gtk::Entry,
    uri_entry: gtk::Entry,
    tags_entry: gtk::Entry,
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

        let title_entry = gtk::Entry::new();
        let uri_entry = gtk::Entry::new();
        let tags_entry = gtk::Entry::new();

        let grid = layout::grid(&[
            &|row| row
                .add_column(&gtk::Label::new("Title"))
                .add_column(&title_entry),
            &|row| row
                .add_column(&gtk::Label::new("URI"))
                .add_column(&uri_entry),
            &|row| row
                .add_column(&gtk::Label::new("Tags"))
                .add_column(&tags_entry),
        ]);
        grid.show_all();
        dialog.get_content_area().add(&grid);

        title_entry.connect_property_text_notify({
            let ok_button = ok_button.clone();
            move |entry| {
                let is_valid = entry
                    .get_text()
                    .map(|text| !text.is_empty())
                    .unwrap_or(false);
                ok_button.set_sensitive(is_valid);
            }
        });

        Dialog {
            dialog,
            ok_button,
            title_entry,
            uri_entry,
            tags_entry,
        }
    }

    fn set(&self, title: &str, uri: &str, tags: &[String]) {
        use gtk::prelude::*;

        let mut tags: Vec<_> = tags.into();
        tags.sort();
        let tags = tags.join(", ");
        self.ok_button.set_sensitive(!title.is_empty());
        self.title_entry.set_text(title);
        self.uri_entry.set_text(uri);
        self.tags_entry.set_text(&tags);
    }

    fn get(&self) -> (String, String, Vec<String>) {
        use gtk::prelude::*;

        (   self.title_entry.get_text().unwrap_or_else(|| String::new()),
            self.uri_entry.get_text().unwrap_or_else(|| String::new()),
            self.tags_entry
                .get_text()
                .map(|text| parse_tags(&text))
                .unwrap_or_else(|| Vec::new()),
        )
    }
}

fn search(app: &app::Handle) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();
    let bookmarks = app.bookmarks();
    let terms = map.search_entry.get_text();
    let terms = terms
        .as_ref()
        .map(|text| text::parse_search(text))
        .unwrap_or_else(|| Vec::new());
    let model = &map.result_list_model;

    let mode = match find_search_mode(app) {
        Some(mode) => mode,
        None => {
            hide_all(&model);
            return;
        },
    };

    let found = bookmarks.search(&terms, mode);
    show_only_contained(&model, &found);
}

fn show_only_contained(model: &gtk::ListStore, ids: &collections::HashSet<BookmarkId>) {
    use gtk::prelude::*;

    for index in 0..model.iter_n_children(None) {
        let iter = model.iter_nth_child(None, index)
            .expect("existing list child");
        let id = model.get_value(&iter, RESULT_COL_ID as i32).get()
            .expect("bookmark id in store");
        model.set(&iter, &[RESULT_COL_VISIBLE], &[&ids.contains(&id)]);
    }
}

fn hide_all(model: &gtk::ListStore) {
    use gtk::prelude::*;

    for index in 0..model.iter_n_children(None) {
        let iter = model.iter_nth_child(None, index).expect("existing list child");
        model.set(&iter, &[RESULT_COL_VISIBLE], &[&false]);
    }
}

fn find_search_mode(app: &app::Handle) -> Option<SearchMode> {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();

    let (paths, model) = map.tag_list.get_selection().get_selected_rows();
    if paths.is_empty() {
        return None;
    }

    let mut untagged = false;
    let mut tags = Vec::new();
    for path in paths {
        let iter = model.get_iter(&path).expect("selected iterator");
        let kind: TagKind = model.get_value(&iter, TAG_COL_KIND as i32).get()
            .expect("tag row kind id");
        match kind {
            TAG_LIST_ALL => return Some(SearchMode::All),
            TAG_LIST_UNTAGGED => untagged = true,
            TAG_LIST_TAG => {
                let tag: String = model.get_value(&iter, TAG_COL_NAME_RAW as i32).get()
                    .expect("tag name");
                tags.push(tag);
            },
            other => panic!("unexpected tag list kind id: {}", other),
        }
    }

    Some(SearchMode::Tagged { tags, untagged })
}

pub fn setup(app: &app::Handle) {
    setup_popover(app);
    setup_panel(app);
}

fn setup_tag_list(app: &app::Handle) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();

    let name_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", TAG_COL_NAME as i32);
        column.set_sort_column_id(TAG_COL_NAME_RAW as i32);
        column.set_expand(true);
        column
    };

    let count_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", TAG_COL_COUNT as i32);
        column.set_expand(false);
        column
    };
    
    map.tag_list.append_column(&name_column);
    map.tag_list.append_column(&count_column);
    map.tag_list.set_headers_visible(false);
    map.tag_list.get_selection().set_mode(gtk::SelectionMode::Multiple);
    map.tag_list.set_model(&map.tag_list_model);
}

fn setup_result_list(app: &app::Handle) {
    use gtk::prelude::*;
    use pango;

    let map = app.stored();
    let map = map.bookmarks();

    let title_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_property_ellipsize(pango::EllipsizeMode::End);
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", RESULT_COL_TITLE as i32);
        column.set_sort_column_id(RESULT_COL_TITLE as i32);
        column.set_expand(true);
        column
    };

    let uri_column = {
        let column = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_property_ellipsize(pango::EllipsizeMode::End);
        column.pack_start(&cell, true);
        column.add_attribute(&cell, "text", RESULT_COL_URI as i32);
        column.set_expand(true);
        column
    };
    
    map.result_list.append_column(&title_column);
    map.result_list.append_column(&uri_column);
    map.result_list.set_tooltip_column(RESULT_COL_URI as i32);
    map.result_list.set_headers_visible(false);
    map.result_list.get_selection().set_mode(gtk::SelectionMode::Single);

    let filter = gtk::TreeModelFilter::new(&map.result_list_model, None);
    filter.set_visible_column(RESULT_COL_VISIBLE as i32);
    map.result_list.set_model(&filter);
}

fn setup_panel(app: &app::Handle) {
    use gtk::prelude::*;
    use layout::{ BuildBox, BuildPaned };

    let map = app.stored();
    let map = map.bookmarks();
    let bookmarks = app.bookmarks();
    let window = app.window();

    setup_tag_list(app);
    setup_result_list(app);

    bookmarks.populate_tags(&map.tag_list_model);
    bookmarks.populate_bookmarks(&map.result_list_model);

    map.edit_button.set_sensitive(false);
    map.remove_button.set_sensitive(false);

    map.add_dialog.dialog.set_transient_for(&window);
    map.edit_dialog.dialog.set_transient_for(&window);

    map.container().add_start_fill(&layout::hpaned(Some(40))
        .add1_secondary(&scrolled::create(map.tag_list.clone()))
        .add2_primary(&layout::vbox()
            .add_start(&map.search_entry)
            .add_start_fill(&scrolled::create(map.result_list.clone()))
            .add_start(&layout::hbox()
                .add_start(&map.add_button)
                .add_start(&map.edit_button)
                .add_start(&map.remove_button)
            )
        )
    );

    for index in 0..map.tag_list_model.iter_n_children(None) {
        let iter = map.tag_list_model.iter_nth_child(None, index).expect("indexed child");
        let kind: TagKind = map.tag_list_model.get_value(&iter, TAG_COL_KIND as i32).get()
            .expect("tag kind");
        if kind == TAG_LIST_ALL {
            map.tag_list.get_selection().select_iter(&iter);
            break;
        }
    }

    map.result_list.get_selection().connect_changed(with_cloned!(app, move |selection| {
        on_bookmark_selection_change(&app, selection);
    }));

    map.search_entry.connect_search_changed(with_cloned!(app, move |_entry| {
        search(&app);
    }));

    map.tag_list.get_selection().connect_changed(with_cloned!(app, move |_selection| {
        search(&app);
    }));

    map.edit_button.connect_clicked(with_cloned!(app, move |_button| {
        edit_selected_bookmark(&app);
    }));

    map.add_button.connect_clicked(with_cloned!(app, move |_button| {
        add_stored_bookmark(&app);
    }));

    map.remove_button.connect_clicked(with_cloned!(app, move |_button| {
        remove_selected_bookmark(&app);
    }));
}

fn on_bookmark_selection_change(app: &app::Handle, selection: &gtk::TreeSelection) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();
    if let Some(_) = selection.get_selected() {
        map.edit_button.set_sensitive(true);
        map.remove_button.set_sensitive(true);
    } else {
        map.edit_button.set_sensitive(false);
        map.remove_button.set_sensitive(false);
    }
}

fn add_stored_bookmark(app: &app::Handle) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();

    let mut tags = Vec::new();
    let (paths, model) = map.tag_list.get_selection().get_selected_rows();
    for path in paths {
        let iter = model.get_iter(&path).expect("resolved path");
        let kind: TagKind = model.get_value(&iter, TAG_COL_KIND as i32).get()
            .expect("tag kind");
        if kind == TAG_LIST_TAG {
            let name: String = model.get_value(&iter, TAG_COL_NAME_RAW as i32).get()
                .expect("raw tag name");
            tags.push(name);
        }
    }

    map.edit_dialog.set("", "", &tags);
    map.edit_dialog.title_entry.grab_focus();

    let result = map.edit_dialog.dialog.run();
    map.edit_dialog.dialog.hide();
    if result == RES_OK {
        let (title, uri, tags) = map.edit_dialog.get();
        add_bookmark(app, &title, &uri, &tags);
    }
}

fn edit_selected_bookmark(app: &app::Handle) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let map = app.stored();
    let map = map.bookmarks();

    let (model, iter) = unwrap_or_return!(map.result_list.get_selection().get_selected());
    let id: BookmarkId = model.get_value(&iter, RESULT_COL_ID as i32).get()
        .expect("stored bookmark id in view model");
    let bookmark = unwrap_or_return!(bookmarks.find_by_id(id));
    map.edit_dialog.set(bookmark.title(), bookmark.uri(), bookmark.tags());
    map.edit_dialog.title_entry.grab_focus();

    let result = map.edit_dialog.dialog.run();
    map.edit_dialog.dialog.hide();
    if result == RES_OK {
        let (title, uri, tags) = map.edit_dialog.get();
        update_bookmark(app, id, &title, &uri, &tags);
    }
}

fn remove_selected_bookmark(app: &app::Handle) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let window = app.window();
    let map = app.stored();
    let map = map.bookmarks();

    let (model, iter) = unwrap_or_return!(map.result_list.get_selection().get_selected());
    let id: BookmarkId = model.get_value(&iter, RESULT_COL_ID as i32).get()
        .expect("stored bookmark id in view model");
    let bookmark = unwrap_or_return!(bookmarks.find_by_id(id));

    let result = window::confirm_action(
        &window,
        &format!("Really remove bookmark '{}'?", bookmark.title()),
        &[("Ok", RES_OK), ("Cancel", RES_CANCEL)],
        RES_OK,
    );
    if result == RES_OK {
        remove_bookmark(app, id);
    }
}

fn setup_popover(app: &app::Handle) {
    use gtk::prelude::*;
    use layout::{ BuildBox };

    let nav_bar = app.navigation_bar();

    let map = app.stored();
    let map = map.bookmarks();

    let dialog = layout::vbox()
        .add_start(&layout::grid(&[
            &|row| row
                .add_column(&gtk::Label::new("Title"))
                .add_column(&map.title_entry),
            &|row| row
                .add_column(&gtk::Label::new("Tags"))
                .add_column(&map.tags_entry),
        ]))
        .add_start(&layout::hbox()
            .add_start(&map.save_current_button)
            .add_start(&map.remove_current_button)
        );
    map.popover.add(&dialog);

    map.popover.set_relative_to(&nav_bar.bookmarks_button());
    map.popover.set_transitions_enabled(false);

    map.remove_current_button.connect_clicked(with_cloned!(app, move |_button| {
        remove_active_bookmark(&app);
    }));

    map.save_current_button.connect_clicked(with_cloned!(app, move |_button| {
        save_active_bookmark(&app);
    }));

    map.title_entry.connect_property_text_notify(with_cloned!(app, move |entry| {
        on_bookmark_title_change(&app, entry);
    }));

    nav_bar.bookmarks_button().connect_clicked(with_cloned!(app, move |_button| {
        show_bookmark_popover(&app);
    }));
}

fn remove_active_bookmark(app: &app::Handle) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let bookmarks = app.bookmarks();
    let webview = unwrap_or_return!(app.active_webview());
    let map = app.stored();
    let map = map.bookmarks();
    let uri = unwrap_or_return!(webview.get_uri());
    let bookmark = bookmarks.find_by_uri(&uri);

    if let Some(bookmark) = bookmark {
        remove_bookmark(app, bookmark.id());
    }
    map.popover.hide();
}

fn remove_bookmark(app: &app::Handle, id: BookmarkId) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let map = app.stored();
    let map = map.bookmarks();
    
    bookmarks.remove_bookmark(id);

    let iter = find_iter(&map.result_list_model, id).expect("bookmark in result list");
    map.result_list_model.remove(&iter);

    recalc_tags(app);
    search(app);
}

fn parse_tags(value: &str) -> Vec<String> {
    value.trim()
        .split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.into())
        .collect()
}

fn save_active_bookmark(app: &app::Handle) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let bookmarks = app.bookmarks();
    let webview = unwrap_or_return!(app.active_webview());
    let map = app.stored();
    let map = map.bookmarks();
    let uri = unwrap_or_return!(webview.get_uri());
    let bookmark = bookmarks.find_by_uri(&uri);

    let title = map.title_entry.get_text().unwrap_or_else(|| String::new());
    let tags = map.tags_entry
        .get_text()
        .map(|text| parse_tags(&text))
        .unwrap_or_else(|| Vec::new());

    if let Some(bookmark) = bookmark {
        update_bookmark(app, bookmark.id(), &title, &uri, &tags);
    } else {
        add_bookmark(app, &title, &uri, &tags);
    }
    map.popover.hide();
}

fn add_bookmark(app: &app::Handle, title: &str, uri: &str, tags: &[String]) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let map = app.stored();
    let map = map.bookmarks();

    let id = bookmarks.add_bookmark(title, uri, tags);

    let title_escaped = text::escape(title);
    let title_escaped: &str = &title_escaped;
    let uri_escaped = text::escape(uri);
    let uri_escaped: &str = &uri_escaped;
    map.result_list_model.insert_with_values(
        None,
        &[RESULT_COL_TITLE, RESULT_COL_URI, RESULT_COL_ID, RESULT_COL_VISIBLE],
        &[&title_escaped, &uri_escaped, &id, &false],
    );
    recalc_tags(app);
    search(app);
}

fn update_bookmark(app: &app::Handle, id: BookmarkId, title: &str, uri: &str, tags: &[String]) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let map = app.stored();
    let map = map.bookmarks();

    bookmarks.update_bookmark(id, title, uri, tags);

    let iter = find_iter(&map.result_list_model, id).expect("bookmark in result list");
    let title_escaped = text::escape(title);
    let title_escaped: &str = &title_escaped;
    let uri_escaped = text::escape(uri);
    let uri_escaped: &str = &uri_escaped;
    map.result_list_model.set(
        &iter,
        &[RESULT_COL_TITLE, RESULT_COL_URI],
        &[&title_escaped, &uri_escaped],
    );
    recalc_tags(app);
    search(app);
}

fn recalc_tags(app: &app::Handle) {
    use gtk::prelude::*;

    let bookmarks = app.bookmarks();
    let map = app.stored();
    let map = map.bookmarks();

    let mut selected = Vec::new();
    let (paths, model) = map.tag_list.get_selection().get_selected_rows();
    for path in paths {
        let iter = model.get_iter(&path).expect("resolved path");
        let kind: TagKind = model.get_value(&iter, TAG_COL_KIND as i32).get()
            .expect("stored tag list kind");
        let name: String = model.get_value(&iter, TAG_COL_NAME_RAW as i32).get()
            .expect("stored tag list name");
        selected.push((kind, name));
    }

    bookmarks.populate_tags(&map.tag_list_model);

    for index in 0..map.tag_list_model.iter_n_children(None) {
        let iter = model.iter_nth_child(None, index).expect("indexed child");
        let kind: TagKind = model.get_value(&iter, TAG_COL_KIND as i32).get()
            .expect("stored tag list kind");
        let name: String = model.get_value(&iter, TAG_COL_NAME_RAW as i32).get()
            .expect("stored tag list name");
        'selected: for &(ref selected_kind, ref selected_name) in &selected {
            if selected_kind == &kind && selected_name == &name {
                map.tag_list.get_selection().select_iter(&iter);
                break 'selected;
            }
        }
    }
}

fn find_iter(model: &gtk::ListStore, search_id: BookmarkId) -> Option<gtk::TreeIter> {
    use gtk::prelude::*;

    for index in 0..model.iter_n_children(None) {
        let iter = model.iter_nth_child(None, index).expect("indexed child");
        let id: BookmarkId = model.get_value(&iter, RESULT_COL_ID as i32).get()
            .expect("bookmark id in model");
        if id == search_id {
            return Some(iter);
        }
    }
    None
}

fn on_bookmark_title_change(app: &app::Handle, entry: &gtk::Entry) {
    use gtk::prelude::*;

    let map = app.stored();
    let map = map.bookmarks();
    let is_empty = entry.get_text().map(|text| text.is_empty()).unwrap_or(true);
    map.save_current_button.set_sensitive(!is_empty);
}

fn show_bookmark_popover(app: &app::Handle) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let bookmarks = app.bookmarks();
    let webview = unwrap_or_return!(app.active_webview());
    let map = app.stored();
    let map = map.bookmarks();
    let uri = unwrap_or_return!(webview.get_uri());
    let bookmark = bookmarks.find_by_uri(&uri);

    map.popover.show_all();
    if let Some(bookmark) = bookmark {
        map.title_entry.set_text(bookmark.title());
        map.tags_entry.set_text(&bookmark.tags().join(", "));
    } else {
        let title = webview.get_title().unwrap_or_else(|| String::new());
        map.title_entry.set_text(&title);
        map.tags_entry.set_text("");
        map.remove_current_button.hide();
    }
    map.tags_entry.grab_focus();
}

enum SearchMode {
    All,
    Tagged { tags: Vec<String>, untagged: bool },
}

pub struct Bookmark {
    id: BookmarkId,
    uri: String,
    title: String,
    tags: Vec<String>,
}

impl Bookmark {

    pub fn id(&self) -> BookmarkId { self.id }
    pub fn uri(&self) -> &str { &self.uri }
    pub fn title(&self) -> &str { &self.title }
    pub fn tags(&self) -> &[String] { &self.tags }
}

pub struct Bookmarks {
    storage: storage::Storage,
}

impl Bookmarks {

    pub fn open_or_create<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {

        let storage = storage::Storage::open_or_create(
            path,
            init_storage,
            storage::do_nothing,
        )?;

        Ok(Bookmarks { storage })
    }

    fn find_by_id(&self, id: BookmarkId) -> Option<Bookmark> {
        self.storage.with_connection(|conn| {
            let mut stmt = conn.prepare("
                SELECT bookmark_id, uri, title
                FROM bookmarks
                WHERE bookmark_id = ?
            ")?;
            let mut rows = stmt.query(&[&id])?;
            let row = match rows.next() {
                Some(row) => row?,
                None => return Ok(None),
            };
            let id: BookmarkId = row.get(0);
            let uri = row.get(1);
            let title = row.get(2);
            let mut stmt = conn.prepare("
                SELECT tag
                FROM tagged_bookmarks
                WHERE bookmark_id = ?
            ")?;
            let mut rows = stmt.query(&[&id])?;
            let mut tags = Vec::new();
            while let Some(row) = rows.next() {
                let row = row?;
                tags.push(row.get(0));
            }
            Ok(Some(Bookmark { id, uri, title, tags }))
        }).expect("bookmark storage query by id")
    }

    fn find_by_uri(&self, uri: &str) -> Option<Bookmark> {
        self.storage.with_connection(|conn| {
            let mut stmt = conn.prepare("
                SELECT bookmark_id, uri, title
                FROM bookmarks
                WHERE uri = ?
            ")?;
            let mut rows = stmt.query(&[&uri])?;
            let row = match rows.next() {
                Some(row) => row?,
                None => return Ok(None),
            };
            let id: BookmarkId = row.get(0);
            let uri = row.get(1);
            let title = row.get(2);
            let mut stmt = conn.prepare("
                SELECT tag
                FROM tagged_bookmarks
                WHERE bookmark_id = ?
            ")?;
            let mut rows = stmt.query(&[&id])?;
            let mut tags = Vec::new();
            while let Some(row) = rows.next() {
                let row = row?;
                tags.push(row.get(0));
            }
            Ok(Some(Bookmark { id, uri, title, tags }))
        }).expect("bookmark storage query by uri")
    }

    fn add_bookmark(&self, title: &str, uri: &str, tags: &[String]) -> BookmarkId {
        self.storage.with_transaction(|tx| {
            tx.execute("
                INSERT INTO bookmarks (uri, title)
                VALUES (?, ?)
            ", &[&uri, &title])?;
            let id = tx.last_insert_rowid();
            let mut stmt = tx.prepare("
                INSERT INTO tagged_bookmarks (bookmark_id, tag)
                VALUES (?, ?)
            ")?;
            for tag in tags {
                stmt.execute(&[&id, &tag.as_str()])?;
            }
            Ok(id)
        }).expect("bookmark storage addition")
    }

    fn remove_bookmark(&self, id: BookmarkId) {
        self.storage.with_transaction(|tx| {
            tx.execute("DELETE FROM bookmarks WHERE bookmark_id = ?", &[&id])?;
            tx.execute("DELETE FROM tagged_bookmarks WHERE bookmark_id = ?", &[&id])?;
            Ok(())
        }).expect("bookmark storage removal")
    }

    fn update_bookmark(&self, id: BookmarkId, title: &str, uri: &str, tags: &[String]) {
        self.storage.with_transaction(|tx| {
            tx.execute("
                UPDATE bookmarks
                SET title = ?, uri = ?
                WHERE bookmark_id = ?
            ", &[&title, &uri, &id])?;
            tx.execute("DELETE FROM tagged_bookmarks WHERE bookmark_id = ?", &[&id])?;
            let mut stmt = tx.prepare("
                INSERT INTO tagged_bookmarks (bookmark_id, tag)
                VALUES (?, ?)
            ")?;
            for tag in tags {
                stmt.execute(&[&id, &tag.as_str()])?;
            }
            Ok(())
        }).expect("bookmark storage update")
    }

    fn search_all_foreach<F>(
        &self,
        cond: &str,
        params: &[String],
        mut callback: F,
    ) -> Result<(), storage::Error> where F: FnMut(BookmarkId) {

        self.storage.with_connection(|conn| {

            let mut stmt = conn.prepare(&format!("
                SELECT bookmark_id
                FROM bookmarks
                WHERE {}
            ", cond))?;

            let params = params.iter()
                .map(|p| p as &rusqlite::types::ToSql)
                .collect::<Vec<_>>();
            let mut rows = stmt.query(&params)?;

            while let Some(row) = rows.next() {
                let row = row?;
                callback(row.get(0));
            }

            Ok(())
        })
    }

    fn search_untagged_foreach<F>(
        &self,
        cond: &str,
        params: &[String],
        mut callback: F,
    ) -> Result<(), storage::Error> where F: FnMut(BookmarkId) {

        self.storage.with_connection(|conn| {

            let mut stmt = conn.prepare(&format!("
                SELECT bookmarks.bookmark_id, COUNT(tagged_bookmarks.tag) AS tag_count
                FROM bookmarks
                LEFT JOIN tagged_bookmarks
                ON bookmarks.bookmark_id = tagged_bookmarks.bookmark_id
                WHERE ({})
                GROUP BY bookmarks.bookmark_id
                HAVING tag_count = 0
            ", cond))?;

            let params = params.iter()
                .map(|p| p as &rusqlite::types::ToSql)
                .collect::<Vec<_>>();
            let mut rows = stmt.query(&params)?;

            while let Some(row) = rows.next() {
                let row = row?;
                callback(row.get(0));
            }

            Ok(())
        })
    }

    fn search_tagged_foreach<F>(
        &self,
        cond: &str,
        params: &[String],
        tags: &[String],
        mut callback: F,
    ) -> Result<(), storage::Error> where F: FnMut(BookmarkId) {

        if tags.is_empty() {
            return Ok(());
        }

        self.storage.with_connection(|conn| {

            let mut tag_cond = String::new();
            let mut tag_params: Vec<_> = params.into();

            for tag in tags {
                if !tag_cond.is_empty() {
                    tag_cond.push_str(" OR ");
                }
                tag_cond.push_str("tagged_bookmarks.tag = ?");
                tag_params.push(tag.clone());
            }

            let mut stmt = conn.prepare(&format!("
                SELECT bookmarks.bookmark_id
                FROM bookmarks, tagged_bookmarks
                WHERE bookmarks.bookmark_id = tagged_bookmarks.bookmark_id
                AND ({})
                AND ({})
                GROUP BY bookmarks.bookmark_id
            ", cond, tag_cond))?;

            let tag_params = tag_params.iter()
                .map(|p| p as &rusqlite::types::ToSql)
                .collect::<Vec<_>>();
            let mut rows = stmt.query(&tag_params)?;

            while let Some(row) = rows.next() {
                let row = row?;
                callback(row.get(0));
            }

            Ok(())
        })
    }

    fn search(&self, terms: &[&str], mode: SearchMode) -> collections::HashSet<BookmarkId> {

        let mut cond = String::new();
        let mut params = Vec::new();

        for part in terms {
            if !cond.is_empty() {
                cond.push_str(" OR ");
            }
            cond.push_str("bookmarks.uri LIKE ? OR bookmarks.title LIKE ?");
            let term = format!("%{}%", part);
            params.push(term.clone());
            params.push(term);
        }

        if cond.is_empty() {
            cond.push_str("1=1");
        }
        
        let mut found = collections::HashSet::new();
        match mode {
            SearchMode::All => {
                self.search_all_foreach(&cond, &params, |id| {
                    found.insert(id);
                }).expect("bookmark storage search on full set");
            },
            SearchMode::Tagged { tags, untagged } => {
                self.search_tagged_foreach(&cond, &params, &tags, |id| {
                    found.insert(id);
                }).expect("bookmark storage search on tagged set");
                if untagged {
                    self.search_untagged_foreach(&cond, &params, |id| {
                        found.insert(id);
                    }).expect("bookmark storage search on untagged set");
                }
            },
        }

        found
    }

    fn populate_bookmarks(&self, model: &gtk::ListStore) {
        use gtk::prelude::*;
        self.storage.with_connection(|conn| {
            model.clear();
            let mut stmt = conn.prepare("
                SELECT bookmark_id, title, uri
                FROM bookmarks
            ")?;
            let mut rows = stmt.query(&[])?;
            while let Some(row) = rows.next() {
                let row = row?;

                let id: BookmarkId = row.get(0);

                let title: String = row.get(1);
                let title_escaped = text::escape(&title);
                let title_escaped: &str = &title_escaped;

                let uri: String = row.get(2);
                let uri_escaped = text::escape(&uri);
                let uri_escaped: &str = &uri_escaped;

                model.insert_with_values(
                    None,
                    &[RESULT_COL_TITLE, RESULT_COL_URI, RESULT_COL_ID, RESULT_COL_VISIBLE],
                    &[&title_escaped, &uri_escaped, &id, &true],
                );
            }
            Ok(())
        }).expect("bookmark storage bookmarks load")
    }

    fn populate_tags(&self, model: &gtk::ListStore) {
        use gtk::prelude::*;
        self.storage.with_connection(|conn| {
            model.clear();
            model.insert_with_values(
                None,
                &[TAG_COL_NAME, TAG_COL_COUNT, TAG_COL_NAME_RAW, TAG_COL_KIND],
                &[&"All", &"", &"", &TAG_LIST_ALL],
            );
            model.insert_with_values(
                None,
                &[TAG_COL_NAME, TAG_COL_COUNT, TAG_COL_NAME_RAW, TAG_COL_KIND],
                &[&"Untagged", &"", &"", &TAG_LIST_UNTAGGED],
            );
            let mut stmt = conn.prepare("
                SELECT tag, COUNT(bookmark_id)
                FROM tagged_bookmarks
                GROUP BY tag
                ORDER BY tag
            ")?;
            let mut rows = stmt.query(&[])?;
            while let Some(row) = rows.next() {
                let row = row?;

                let name: String = row.get(0);
                let name_escaped = text::escape(&name);
                let name_escaped: &str = &name_escaped;

                let count: i64 = row.get(1);

                model.insert_with_values(
                    None,
                    &[TAG_COL_NAME, TAG_COL_COUNT, TAG_COL_NAME_RAW, TAG_COL_KIND],
                    &[&name_escaped, &count, &name, &TAG_LIST_TAG],
                );
            }
            Ok(())
        }).expect("bookmark storage tags load")
    }
}

fn init_storage(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("
        CREATE TABLE bookmarks (
            bookmark_id INTEGER PRIMARY KEY NOT NULL,
            uri TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL
        )
    ", &[])?;
    conn.execute("
        CREATE TABLE tagged_bookmarks (
            bookmark_id INTEGER,
            tag TEXT NOT NULL
        )
    ", &[])?;
    Ok(())
}

