
use std::rc;
use std::cell;
use std::cmp;
use std::collections;

use gtk;
use webkit2gtk;
use cairo;
use pango;

use webview;
use app;
use page_tree_store;
use session;
use recently_closed;
use text;
use signal;
use script_dialog;

pub type Id = u32;

const TITLE_WEIGHT_DEFAULT: i32 = 400;
const TITLE_WEIGHT_UNREAD: i32 = 600;

#[derive(Debug, Clone, Copy)]
pub struct LoadState {
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub tls_state: TlsState,
    pub event: Option<webkit2gtk::LoadEvent>,
}

#[derive(Debug, Clone, Copy)]
pub enum TlsState {
    Insecure,
    SelfSigned,
    Encrypted,
}

pub struct Store {
    last_id: cell::Cell<Id>,
    entries: rc::Rc<cell::RefCell<collections::HashMap<Id, Entry>>>,
    pinned: cell::RefCell<Vec<Id>>,
    tree_store: gtk::TreeStore,
    session: Option<session::Session>,
    recently_closed: recently_closed::State,
    count_change_notifier: signal::Notifier<Store, usize>,
    load_state_change_notifier: signal::Notifier<Store, (Id, LoadState)>,
}

pub fn setup(app: &app::Handle) {

    let page_tree_view = app.page_tree_view();
    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {
        let page_store = app.page_store();
        page_store.set_read(id);
        page_store.session.as_ref().map(|session| session.update_selected(id));
    }));
}

impl Store {

    fn_connect_notifier!(count_change_notifier, on_count_change, usize);
    fn_connect_notifier!(load_state_change_notifier, on_load_state_change, (Id, LoadState));

    pub fn new_stateful(session: session::Session) -> (Store, Option<Id>) {
        log_debug!("new stateful page store");

        fn populate(
            parent: Option<&gtk::TreeIter>,
            children: &[session::Node],
            entries: &mut collections::HashMap<Id, Entry>,
            tree_store: &gtk::TreeStore,
        ) {
            for child in children {
                let id = child.id();
                entries.insert(id, Entry {
                    id,
                    uri: child.uri().clone(),
                    title: child.title().cloned(),
                    view: None,
                    favicon: None,
                    is_noclose: false,
                    is_pinned: child.is_pinned(),
                    load_state: LoadState {
                        can_go_back: false,
                        can_go_forward: false,
                        is_loading: false,
                        tls_state: TlsState::Insecure,
                        event: None,
                    },
                });
                let iter = page_tree_store::insert(
                    tree_store,
                    parent,
                    None,
                    page_tree_store::Entry {
                        id,
                        title: text::escape(child.title().unwrap_or_else(|| child.uri())).into(),
                        child_info: "".into(),
                        has_children: false,
                        child_count: 0,
                        style: pango::Style::Italic,
                        weight: TITLE_WEIGHT_DEFAULT,
                        is_pinned: child.is_pinned(),
                    },
                );
                populate(Some(&iter), &child.children(), entries, tree_store);
                page_tree_store::recalc(tree_store, &iter);
            }
        }

        let mut tree = session.load_tree()
            .expect("session tree loaded from storage");
        tree.compact();
        let last_id = tree.find_highest_id().unwrap_or(0).checked_add(1)
            .expect("last id in available id space");
        let pinned = tree.find_pinned();
        let last_selected = tree.find_selected();
        if let Some(last_selected) = last_selected {
            session.update_selected(last_selected)
                .expect("updated selected page in session for adjusted tree");
        }

        let mut entries = collections::HashMap::new();
        let tree_store = page_tree_store::create();

        populate(None, tree.children(), &mut entries, &tree_store);

        let store = Store {
            last_id: cell::Cell::new(last_id),
            entries: rc::Rc::new(cell::RefCell::new(entries)),
            tree_store,
            pinned: cell::RefCell::new(pinned),
            session: Some(session),
            recently_closed: recently_closed::State::new(),
            count_change_notifier: signal::Notifier::new(),
            load_state_change_notifier: signal::Notifier::new(),
        };
        store.update_session();

        (store, last_selected)
    }

    pub fn new_stateless() -> Store {
        log_debug!("new stateless page store");
        Store {
            last_id: cell::Cell::new(0),
            entries: rc::Rc::new(cell::RefCell::new(collections::HashMap::new())),
            tree_store: page_tree_store::create(),
            pinned: cell::RefCell::new(Vec::new()),
            session: None,
            recently_closed: recently_closed::State::new(),
            count_change_notifier: signal::Notifier::new(),
            load_state_change_notifier: signal::Notifier::new(),
        }
    }

    pub fn recently_closed_state(&self) -> &recently_closed::State { &self.recently_closed }

    pub fn update_session(&self) {
        self.session
            .as_ref()
            .map(|session| session.update_all(self).expect("session update"));
    }

    pub fn update_session_node(&self, id: Id) {
        self.session
            .as_ref()
            .map(|session| session.update_node(self, id).expect("node session update"));
    }

    pub fn pinned_count(&self) -> usize { self.pinned.borrow().len() }

    pub fn tree_store(&self) -> &gtk::TreeStore { &self.tree_store }

    pub fn set_unread(&self, id: Id) {
        let iter = unwrap_or_return!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        page_tree_store::set_weight(&self.tree_store, &iter, TITLE_WEIGHT_UNREAD);
    }

    pub fn set_read(&self, id: Id) {
        let iter = unwrap_or_return!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        page_tree_store::set_weight(&self.tree_store, &iter, TITLE_WEIGHT_DEFAULT);
    }

    pub fn set_pinned(&self, id: Id, is_pinned: bool) {
        use gtk::prelude::*;

        let iter = unwrap_or_return!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let old_parent = self.tree_store.iter_parent(&iter);

        self.map_entry_mut(id, |entry| entry.is_pinned = is_pinned);
        page_tree_store::set_is_pinned(&self.tree_store, &iter, is_pinned);
        let count = self.pinned.borrow().len();
        if is_pinned {
            self.move_to(id, None, count as i32);
            self.pinned.borrow_mut().push(id);
        } else {
            self.move_to(id, None, count as i32);
            self.pinned.borrow_mut().retain(|pinned_id| *pinned_id != id);
        }
        self.update_session();
        if let Some(old_parent) = old_parent {
            self.recalc(&old_parent);
        }
    }

    pub fn get_pinned(&self, id: Id) -> bool {
        self.map_entry(id, |entry| entry.is_pinned).unwrap_or(false)
    }

    pub fn get_noclose(&self, id: Id) -> bool {
        self.map_entry(id, |entry| entry.is_noclose).unwrap_or(false)
    }

    pub fn set_noclose(&self, id: Id, is_noclose: bool) {
        self.map_entry_mut(id, |entry| entry.is_noclose = is_noclose);
    }

    pub fn get_count(&self) -> usize { self.entries.borrow().len() }

    pub fn get_position_profile(&self, id: Id) -> Vec<(Option<Id>, u32)> {
        log_trace!("get position profile for {}", id);
        
        let mut parents = Vec::new();

        let mut current = id;
        loop {
            match self.position(current) {
                Some((Some(parent), position)) => {
                    log_trace!("child of {} at position {}", parent, position);
                    parents.push((Some(parent), position));
                    current = parent;
                },
                Some((None, position)) => {
                    log_trace!("root node at position {}", position);
                    parents.push((None, position));
                    break;
                },
                None => break,
            }
        }

        log_trace!("position profile complete");
        parents
    }

    pub fn get_parent(&self, id: Id) -> Option<Id> {
        use gtk::prelude::*;

        let iter = page_tree_store::find_iter_by_id(&self.tree_store, id)?;
        let parent_iter = self.tree_store.iter_parent(&iter)?;
        Some(page_tree_store::get_id(&self.tree_store, &parent_iter))
    }

    pub fn position(&self, id: Id) -> Option<(Option<Id>, u32)> {
        let iter = page_tree_store::find_iter_by_id(&self.tree_store, id)?;
        page_tree_store::find_position(&self.tree_store, &iter).map(|(parent, position)| (
            parent.map(|iter| page_tree_store::get_id(&self.tree_store, &iter)),
            position,
        ))
    }

    pub fn nth_child(&self, parent: Option<Id>, index: u32) -> Option<Id> {
        use gtk::prelude::*;

        let parent_iter = parent.map(|id| {
            page_tree_store::find_iter_by_id(&self.tree_store, id)
                .expect("parent id is valid")
        });
        let child_iter = match self.tree_store.iter_nth_child(parent_iter.as_ref(), index as i32) {
            Some(child) => child,
            None => return None,
        };
        let child_id: Id = page_tree_store::get_id(&self.tree_store, &child_iter);
        Some(child_id)
    }

    pub fn has_children(&self, id: Id) -> Option<i32> {
        use gtk::prelude::*;

        let iter = page_tree_store::find_iter_by_id(&self.tree_store, id)?;
        let count = self.tree_store.iter_n_children(Some(&iter));
        if count > 0 {
            Some(count)
        } else {
            None
        }
    }

    pub fn find_next_incl(&self, parent: Option<Id>, position: u32) -> Option<Id> {

        let mut position = position;
        let mut parent = parent;

        loop {
            if let Some(next_id) = self.nth_child(parent, position) {
                return Some(next_id);
            }
            if let Some(parent_id) = parent {
                let (parent_parent, parent_position) = self.position(parent_id)?;
                parent = parent_parent;
                position = parent_position + 1;
                continue;
            }
            return None;
        }
    }

    pub fn find_previous(&self, parent: Option<Id>, position: u32) -> Option<Id> {

        if position == 0 {
            if let Some(parent_id) = parent {
                return Some(parent_id);
            }
            return None;
        }

        if let Some(prev_id) = self.nth_child(parent, position - 1) {
            return Some(prev_id);
        }

        None
    }

    pub fn children(&self, parent: Option<&gtk::TreeIter>) -> Vec<(Id, gtk::TreeIter)> {
        use gtk::prelude::*;

        let mut children = Vec::new();
        let count = self.tree_store.iter_n_children(parent);
        for index in 0..count {
            let child = self.tree_store.iter_nth_child(parent, index).expect("child iter");
            let child_id: Id = page_tree_store::get_id(&self.tree_store, &child);
            children.push((child_id, child));
        }
        children
    }

    pub fn exists(&self, id: Id) -> bool {
        self.entries.borrow().contains_key(&id)
    }

    pub fn close(&self, id: Id, close_children: bool) {
        use gtk::prelude::*;
        use dynamic::{ BorrowMutIn };

        log_debug!("closing page {} (close children: {:?})", id, close_children);

        fn deep_close(
            page_store: &Store,
            store: &gtk::TreeStore,
            iter: &gtk::TreeIter,
        ) {
            let id = page_tree_store::get_id(store, &iter);
            for (_child_id, child_iter) in page_store.children(Some(iter)) {
                deep_close(page_store, store, &child_iter);
            }

            let entry = page_store.entries.borrow_mut_in(|mut entries| {
                entries.remove(&id).expect("page removed from storage")
            });
            page_store.recently_closed.push(recently_closed::Page {
                id,
                title: entry.title,
                uri: entry.uri,
                position: page_store.get_position_profile(id),
            });
            store.remove(iter);

            let webview = match entry.view {
                Some(webview) => webview,
                None => return,
            };

            let mut widget: gtk::Widget = webview.upcast();
            while let Some(parent) = widget.get_parent() {
                if let Some(name) = gtk::WidgetExt::get_name(&parent) {
                    if &name == "view-space" {
                        let view_space = parent.downcast::<gtk::Container>()
                            .expect("view-space to gtk::Container");
                        view_space.remove(&widget);
                        break;
                    }
                }
                widget = parent;
            }
        }
        
        let iter = unwrap_or_return!(page_tree_store::find_iter_by_id(&self.tree_store, id));

        if !close_children {
            let (curr_parent, curr_position) = unwrap_or_return!(self.position(id));
            let children = self.children(Some(&iter));
            for child_index in 0..children.len() {
                let &(child_id, _) = &children[child_index];
                self.move_to(
                    child_id,
                    curr_parent,
                    (curr_position + child_index as u32) as i32,
                );
            }
        }

        let parent_iter = self.tree_store.iter_parent(&iter);

        deep_close(self, &self.tree_store, &iter);
        self.update_session();
        self.count_change_notifier.emit(self, &self.get_count());

        if let Some(parent_iter) = parent_iter {
            self.recalc(&parent_iter);
        }
    }

    fn move_iter_to(
        &self,
        iter: gtk::TreeIter,
        parent_iter: Option<&gtk::TreeIter>,
        position: i32,
    ) {
        use gtk::prelude::*;

        let mut values = Vec::new();
        for column in 0..self.tree_store.get_n_columns() {
            values.push(self.tree_store.get_value(&iter, column));
        }
        
        let new_iter = self.tree_store.insert(parent_iter, position);
        let mut child_index = 0;
        for (_, child_iter) in self.children(Some(&iter)) {
            self.move_iter_to(child_iter, Some(&new_iter), child_index);
            child_index += 1;
        }
        self.tree_store.remove(&iter);

        for column in 0..values.len() {
            self.tree_store.set_value(&new_iter, column as u32, &values[column]);
        }
    }

    pub fn move_to(&self, id: Id, parent: Option<Id>, position: i32) {

        let iter = unwrap_or_return!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let parent_iter = parent.map(|id| {
            page_tree_store::find_iter_by_id(&self.tree_store, id)
                .expect("parent id is valid")
        });

        self.move_iter_to(iter, parent_iter.as_ref(), position);
    }

    pub fn get_uri(&self, id: Id) -> Option<text::RcString> {
        self.map_entry(id, |entry| entry.uri.clone())
    }

    pub fn get_title(&self, id: Id) -> Option<text::RcString> {
        self.map_entry(id, |entry| match entry.title {
            Some(ref title) => title.clone(),
            None => entry.uri.clone(),
        })
    }

    fn update_tree_title(&self, id: Id) {
        if let Some(iter) = page_tree_store::find_iter_by_id(&self.tree_store, id) {
            let title = self.get_title(id).map(|title| (title.len(), title));
            match title {
                Some((len, ref title)) if len > 0 => {
                    page_tree_store::set_title(
                        &self.tree_store,
                        &iter,
                        &text::escape(&title),
                    );
                },
                _ => {
                    page_tree_store::set_title(
                        &self.tree_store,
                        &iter,
                        &text::escape(&self.get_uri(id).unwrap_or_else(|| text::RcString::new())),
                    );
                },
            };
        }
    }

    pub fn set_uri(&self, id: Id, value: text::RcString) {
        self.map_entry_mut(id, |entry| entry.uri = value);
        self.update_tree_title(id);
        self.update_session_node(id);
    }

    pub fn set_title(&self, id: Id, value: Option<text::RcString>) {
        self.map_entry_mut(id, |entry| entry.title = value);
        self.update_tree_title(id);
        self.update_session_node(id);
    }

    pub fn get_load_state(&self, id: Id) -> Option<LoadState> {
        self.map_entry(id, |entry| entry.load_state)
    }

    pub fn set_load_state(&self, id: Id, state: LoadState) {
        self.map_entry_mut(id, |entry| entry.load_state = state);
        self.load_state_change_notifier.emit(self, &(id, state));
    }

    pub fn get_favicon(&self, id: Id) -> Option<cairo::Surface> {
        match self.map_entry(id, |entry| entry.favicon.clone()) {
            Some(Some(favicon)) => Some(favicon),
            _ => None,
        }
    }

    pub fn set_favicon(&self, id: Id, favicon: Option<cairo::Surface>) {
        self.map_entry_mut(id, |entry| entry.favicon = favicon);
    }

    fn map_entry_mut<F, R>(&self, id: Id, callback: F) -> Option<R>
    where F: FnOnce(&mut Entry) -> R {
        for entry in self.entries.borrow_mut().values_mut() {
            if entry.id == id {
                return Some(callback(entry));
            }
        }
        None
    }

    fn map_entry<F, R>(&self, id: Id, callback: F) -> Option<R>
    where F: FnOnce(&Entry) -> R {
        for entry in self.entries.borrow().values() {
            if entry.id == id {
                return Some(callback(entry));
            }
        }
        None
    }

    pub fn get_data(&self, id: Id) -> Option<Data> {
        self.map_entry(id, |entry| Data {
            id,
            title: entry.title.clone(),
            uri: entry.uri.clone(),
            is_pinned: entry.is_pinned,
        })
    }

    pub fn try_get_view(&self, id: Id) -> Option<webkit2gtk::WebView> {
        self.map_entry(id, |entry| entry.view.clone()).and_then(|view| view)
    }

    pub fn get_view(&self, id: Id, app: &app::Handle) -> Option<webkit2gtk::WebView> {
        use webkit2gtk::{ WebViewExt };

        match self.map_entry(id, |entry| entry.view.clone())? {
            Some(view) => return Some(view),
            None => (),
        };

        log_debug!("creating webview for page {}", id);

        let uri = self.map_entry(id, |entry| entry.uri.clone())?;
        let new_view = webview::create(id, app);
        script_dialog::connect(app, &new_view);
        new_view.load_uri(&uri);
        let ret_view = new_view.clone();
        self.map_entry_mut(id, move |entry| entry.view = Some(new_view));
        let iter = page_tree_store::find_iter_by_id(&self.tree_store, id)
            .expect("view id is valid");
        page_tree_store::set_style(&self.tree_store, &iter, pango::Style::Normal);
        Some(ret_view)
    }

    fn find_next_id(&self) -> Id {
        let id = self.last_id.get();
        let next_id = id.checked_add(1).expect("left-over id space");
        self.last_id.set(next_id);
        next_id
    }

    pub fn insert(&self, data: InsertData) -> Option<Id> {
        use gtk::prelude::*;

        let InsertData { uri, title, parent, position, reuse_id } = data;

        let id = reuse_id.unwrap_or_else(|| self.find_next_id());
        if self.exists(id) {
            panic!("page id {} is already in store", id);
        }

        log_debug!("insert page {}: {:?}", id, uri.as_str());
        let parent_iter = match data.parent {
            Some(parent_id) => Some(page_tree_store::find_iter_by_id(
                &self.tree_store,
                parent_id,
            )?),
            None => None,
        };
        self.entries.borrow_mut().insert(id, Entry {
            id,
            uri: uri.clone(),
            title: title.clone(),
            view: None,
            favicon: None,
            is_noclose: false,
            is_pinned: false,
            load_state: LoadState {
                can_go_back: false,
                can_go_forward: false,
                is_loading: false,
                tls_state: TlsState::Insecure,
                event: None,
            },
        });
        let position = {
            let end_index = self.tree_store.iter_n_children(parent_iter.as_ref()) as u32;
            let mut position = match position {
                InsertPosition::Start => 0,
                InsertPosition::End => end_index,
                InsertPosition::At(index) => index,
                InsertPosition::Before(id) => self.position(id)
                    .and_then(|(position_parent, position)| {
                        if position_parent == parent {
                            Some(position)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(end_index),
                InsertPosition::After(id) => self.position(id)
                    .and_then(|(position_parent, position)| {
                        if position_parent == parent {
                            Some(position + 1)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(end_index),
            };
            if parent_iter.is_none() {
                let pin_count = self.pinned.borrow().len() as u32;
                position = cmp::max(position, pin_count);
            }
            position = cmp::min(position, end_index + 1);
            position
        };
        let title = title.unwrap_or_else(|| text::RcString::new());
        page_tree_store::insert(
            &self.tree_store,
            parent_iter.as_ref(),
            Some(position),
            page_tree_store::Entry {
                id,
                title: text::escape(&title).into(),
                child_info: "".into(),
                has_children: false,
                child_count: 0,
                style: pango::Style::Italic,
                weight: TITLE_WEIGHT_DEFAULT,
                is_pinned: false,
            },
        );
        self.update_session();
        self.count_change_notifier.emit(self, &self.get_count());
        if let Some(ref iter) = parent_iter {
            self.recalc(iter);
        }
        Some(id)
    }

    fn recalc(&self, iter: &gtk::TreeIter) {
        page_tree_store::recalc(&self.tree_store, iter);
    }

    pub fn contains(&self, id: Id) -> bool {
        self.map_entry(id, |_| ()).is_some()
    }
}

#[derive(Debug)]
pub enum InsertPosition {
    Start,
    At(u32),
    Before(Id),
    After(Id),
    End,
}

#[derive(Debug)]
pub struct InsertData {
    pub uri: text::RcString,
    pub title: Option<text::RcString>,
    pub parent: Option<Id>,
    pub position: InsertPosition,
    pub reuse_id: Option<Id>,
}

impl InsertData {

    pub fn new(uri: text::RcString) -> InsertData {
        InsertData {
            uri,
            title: None,
            parent: None,
            position: InsertPosition::Start,
            reuse_id: None,
        }
    }

    pub fn with_title(mut self, title: Option<text::RcString>) -> Self {
        self.title = title;
        self
    }

    pub fn with_parent(mut self, parent: Option<Id>) -> Self {
        self.parent = parent;
        self
    }

    pub fn with_position(mut self, position: InsertPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_reused_id(mut self, id: Id) -> Self {
        self.reuse_id = Some(id);
        self
    }
}

#[derive(Debug)]
struct Entry {
    id: Id,
    uri: text::RcString,
    title: Option<text::RcString>,
    view: Option<webkit2gtk::WebView>,
    load_state: LoadState,
    favicon: Option<cairo::Surface>,
    is_pinned: bool,
    is_noclose: bool,
}

#[derive(Debug)]
pub struct Data {
    pub id: Id,
    pub title: Option<text::RcString>,
    pub uri: text::RcString,
    pub is_pinned: bool,
}
