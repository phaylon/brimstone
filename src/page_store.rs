
use std::rc;
use std::cell;
use std::cmp;

use gtk;
use webkit2gtk;
use cairo;
use pango;

use webview;
use app;
use page_tree_store;
use session;
use recently_closed;

pub type Id = u32;

const TITLE_WEIGHT_DEFAULT: i32 = 400;
const TITLE_WEIGHT_UNREAD: i32 = 600;

#[derive(Debug, Clone, Copy)]
pub struct LoadState {
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
    pub tls_state: TlsState,
}

#[derive(Debug, Clone, Copy)]
pub enum TlsState {
    Insecure,
    SelfSigned,
    Encrypted,
}

pub struct Store {
    last_id: cell::Cell<Id>,
    entries: rc::Rc<cell::RefCell<Vec<Entry>>>,
    pinned: cell::RefCell<Vec<Id>>,
    tree_store: gtk::TreeStore,
    session: Option<session::Updater>,
    recently_closed: recently_closed::State,
}

pub fn setup(_app: app::Handle) {}

fn recalc(tree_store: &gtk::TreeStore, iter: &gtk::TreeIter) {
    use gtk::{ TreeModelExt, Cast };

    let model = tree_store.clone().upcast();
    
    let mut sum = 0;
    for index in 0..tree_store.iter_n_children(iter) {
        let child_iter = tree_store.iter_nth_child(iter, index).unwrap();
        sum += 1 + page_tree_store::get::child_count(&model, &child_iter);
    }

    page_tree_store::set::child_count(tree_store, iter, sum);
    page_tree_store::set::has_children(tree_store, iter, sum > 0);
    page_tree_store::set::child_info(tree_store, iter, format!("({})", sum));

    if let Some(parent_iter) = tree_store.iter_parent(iter) {
        recalc(tree_store, &parent_iter);
    }
}

impl Store {

    pub fn new_stateful(mut session: session::Storage) -> Store {
        log_debug!("constructing store from session");

        fn populate(
            parent: Option<&gtk::TreeIter>,
            children: &session::Nodes,
            entries: &mut Vec<Entry>,
            tree_store: &gtk::TreeStore,
        ) {
            for child in children {
                let info = child.borrow().info.clone().unwrap_or_else(|| session::NodeInfo {
                    title: Some(String::new()),
                    uri: String::new(),
                    is_pinned: false,
                });
                let session::NodeInfo { title, uri, is_pinned } = info;
                entries.push(Entry {
                    id: child.borrow().id,
                    uri: uri.clone(),
                    title: title.clone(),
                    view: None,
                    favicon: None,
                    is_noclose: false,
                    is_pinned: is_pinned,
                    load_state: LoadState {
                        can_go_back: false,
                        can_go_forward: false,
                        is_loading: false,
                        tls_state: TlsState::Insecure,
                    },
                });
                let iter = page_tree_store::insert(
                    tree_store,
                    parent.map(|iter| iter.clone()),
                    None,
                    page_tree_store::Entry {
                        id: child.borrow().id,
                        title: title.unwrap_or_else(|| uri),
                        child_info: "".into(),
                        has_children: false,
                        child_count: 0,
                        style: pango::Style::Italic,
                        weight: TITLE_WEIGHT_DEFAULT,
                        is_pinned: is_pinned,
                    },
                );
                populate(Some(&iter), &child.borrow().children, entries, tree_store);
                recalc(tree_store, &iter);
            }
        }

        let last_id = session.find_highest_id().checked_add(1).unwrap();
        let pinned = session.find_pinned_ids();
        let tree = session.load_tree().unwrap();

        let mut entries = Vec::new();
        let tree_store = page_tree_store::create();

        log_debug!("populating store from session");
        populate(None, &tree, &mut entries, &tree_store);
    
        let session_updater = session::Updater::new(session);

        Store {
            last_id: cell::Cell::new(last_id),
            entries: rc::Rc::new(cell::RefCell::new(entries)),
            tree_store,
            pinned: cell::RefCell::new(pinned),
            session: Some(session_updater),
            recently_closed: recently_closed::State::new(),
        }
    }

    pub fn new_stateless() -> Store {
        Store {
            last_id: cell::Cell::new(0),
            entries: rc::Rc::new(cell::RefCell::new(Vec::new())),
            tree_store: page_tree_store::create(),
            pinned: cell::RefCell::new(Vec::new()),
            session: None,
            recently_closed: recently_closed::State::new(),
        }
    }

    pub fn session_update_tree(&self) {
        let tree_store = &self.tree_store;
        self.session_update(|session| session.update_tree(tree_store));
    }

    pub fn pinned_count(&self) -> usize { self.pinned.borrow().len() }

    pub fn tree_store(&self) -> &gtk::TreeStore { &self.tree_store }

    pub fn set_unread(&self, id: Id) {
        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        page_tree_store::set::weight(&self.tree_store, &iter, TITLE_WEIGHT_UNREAD);
    }

    pub fn set_read(&self, id: Id) {
        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        page_tree_store::set::weight(&self.tree_store, &iter, TITLE_WEIGHT_DEFAULT);
    }

    fn session_update<F>(&self, body: F) where F: FnOnce(&session::Updater) {
        if let Some(ref updater) = self.session {
            body(updater);
        }
    }

    pub fn set_pinned(&self, id: Id, is_pinned: bool) {
        use gtk::{ TreeModelExt };

        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let old_parent = self.tree_store.iter_parent(&iter);

        self.map_entry_mut(id, |entry| entry.is_pinned = is_pinned);
        page_tree_store::set::is_pinned(&self.tree_store, &iter, is_pinned);
        let count = self.pinned.borrow().len();
        if is_pinned {
            self.move_to(id, None, count as i32);
            self.pinned.borrow_mut().push(id);
        } else {
            self.move_to(id, None, count as i32);
            self.pinned.borrow_mut().retain(|pinned_id| *pinned_id != id);
        }
        self.session_update(|session| {
            session.update_tree(&self.tree_store);
            session.update_is_pinned(id, is_pinned);
        });
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
        use gtk::{ TreeModelExt, Cast };

        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let parent_iter = try_extract!(self.tree_store.iter_parent(&iter));
        let model = self.tree_store.clone().upcast();
        Some(page_tree_store::get::id(&model, &parent_iter))
    }

    pub fn position(&self, id: Id) -> Option<(Option<Id>, u32)> {
        let parent = self.get_parent(id);
        let position = try_extract!(page_tree_store::find_position(&self.tree_store, id, parent));
        Some((parent, position))
    }

    pub fn nth_child(&self, parent: Option<Id>, index: u32) -> Option<Id> {
        use gtk::{ TreeModelExt, Cast };

        let model: gtk::TreeModel = self.tree_store.clone().upcast();
        let parent_iter = parent
            .map(|id| page_tree_store::find_iter_by_id(&self.tree_store, id).unwrap());
        let child_iter = match model.iter_nth_child(parent_iter.as_ref(), index as i32) {
            Some(child) => child,
            None => return None,
        };
        let child_id: Id = page_tree_store::get::id(&model, &child_iter);
        Some(child_id)
    }

    pub fn has_children(&self, id: Id) -> Option<i32> {
        use gtk::{ TreeModelExt };

        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
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
                let (parent_parent, parent_position) = try_extract!(self.position(parent_id));
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
        use gtk::{ TreeModelExt, Cast };

        let model: gtk::TreeModel = self.tree_store.clone().upcast();
        let mut children = Vec::new();
        let count = model.iter_n_children(parent);
        for index in 0..count {
            let child = model.iter_nth_child(parent, index).expect("child iter");
            let child_id: Id = page_tree_store::get::id(&model, &child);
            children.push((child_id, child));
        }
        children
    }

    pub fn close(&self, id: Id, close_children: bool) {
        use gtk::{ Cast, TreeStoreExt, ContainerExt, WidgetExt, TreeModelExt };

        fn deep_close(
            page_store: &Store,
            model: &gtk::TreeModel,
            store: &gtk::TreeStore,
            iter: &gtk::TreeIter,
        ) {
            let id: Id = page_tree_store::get::id(&model, &iter);
            for (_child_id, child_iter) in page_store.children(Some(iter)) {
                deep_close(page_store, model, store, &child_iter);
            }

            let mut entries = page_store.entries.borrow_mut();
            let index = entries.iter().position(|entry| entry.id == id).unwrap();
            let entry = entries.remove(index);
            page_store.recently_closed.push(recently_closed::Page {
                id,
                title: entry.title,
                uri: entry.uri,
                position: page_store.get_position_profile(id),
            });
            store.remove(iter);
            page_store.session_update(|session| session.update_remove(id));

            let webview = match entry.view {
                Some(webview) => webview,
                None => return,
            };

            let mut widget: gtk::Widget = webview.upcast();
            while let Some(parent) = widget.get_parent() {
                if let Some(name) = parent.get_name() {
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
        
        let model = self.tree_store.clone().upcast();
        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));

        if !close_children {
            let (curr_parent, curr_position) = try_extract!(self.position(id));
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

        deep_close(self, &model, &self.tree_store, &iter);
        self.session_update(|session| session.update_tree(&self.tree_store));

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
        use gtk::{ TreeStoreExt, TreeModelExt, TreeStoreExtManual };

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

        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let parent_iter = parent
            .map(|id| page_tree_store::find_iter_by_id(&self.tree_store, id).unwrap());

        self.move_iter_to(iter, parent_iter.as_ref(), position);
    }

    pub fn get_uri(&self, id: Id) -> Option<String> {
        self.map_entry(id, |entry| entry.uri.clone())
    }

    pub fn get_title(&self, id: Id) -> Option<String> {
        self.map_entry(id, |entry| match entry.title {
            Some(ref title) => title.clone(),
            None => entry.uri.clone(),
        })
    }

    fn update_tree_title(&self, id: Id) {
        if let Some(iter) = page_tree_store::find_iter_by_id(&self.tree_store, id) {
            let title = self.get_title(id);
            page_tree_store::set::title(&self.tree_store, &iter, match title {
                Some(ref title) =>
                    if title.is_empty() {
                        self.get_uri(id).unwrap_or_else(|| String::new())
                    } else {
                        title.clone()
                    },
                None => self.get_uri(id).unwrap_or_else(|| String::new()),
            });
        }
    }

    pub fn set_uri(&self, id: Id, value: String) {
        self.session_update(|session| session.update_uri(id, value.clone()));
        self.map_entry_mut(id, |entry| entry.uri = value);
        self.update_tree_title(id);
    }

    pub fn set_title(&self, id: Id, value: Option<String>) {
        self.session_update(|session|
            session.update_title(id, value.clone().unwrap_or_else(|| String::new()))
        );
        self.map_entry_mut(id, |entry| entry.title = value);
        self.update_tree_title(id);
    }

    pub fn get_load_state(&self, id: Id) -> Option<LoadState> {
        self.map_entry(id, |entry| entry.load_state)
    }

    pub fn set_load_state(&self, id: Id, state: LoadState) {
        self.map_entry_mut(id, |entry| entry.load_state = state);
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
        for entry in self.entries.borrow_mut().iter_mut() {
            if entry.id == id {
                return Some(callback(entry));
            }
        }
        None
    }

    fn map_entry<F, R>(&self, id: Id, callback: F) -> Option<R>
    where F: FnOnce(&Entry) -> R {
        for entry in self.entries.borrow().iter() {
            if entry.id == id {
                return Some(callback(entry));
            }
        }
        None
    }

    pub fn try_get_view(&self, id: Id) -> Option<webkit2gtk::WebView> {
        self.map_entry(id, |entry| entry.view.clone()).and_then(|view| view)
    }

    pub fn get_view(&self, id: Id, app: &app::Handle) -> Option<webkit2gtk::WebView> {
        use webkit2gtk::{ WebViewExt };

        let view = self.map_entry(id, |entry| entry.view.clone());
        match view {
            Some(Some(view)) => return Some(view),
            Some(None) => (),
            None => return None,
        };

        let uri = self.map_entry(id, |entry| entry.uri.clone()).unwrap();
        let new_view = webview::create(id, app);
        new_view.load_uri(&uri);
        let ret_view = new_view.clone();
        self.map_entry_mut(id, move |entry| entry.view = Some(new_view));
        let iter = page_tree_store::find_iter_by_id(&self.tree_store, id).unwrap();
        page_tree_store::set::style(&self.tree_store, &iter, pango::Style::Normal);
        Some(ret_view)
    }

    fn find_next_id(&self) -> Id {
        let id = self.last_id.get();
        let next_id = id.checked_add(1).expect("left-over id space");
        self.last_id.set(next_id);
        next_id
    }

    pub fn insert(&self, data: InsertData) -> Option<Id> {
        use gtk::{ TreeModelExt };
        let InsertData { uri, title, parent, position } = data;

        let id = self.find_next_id();
        let parent_iter = match data.parent {
            Some(parent_id) => Some(try_extract!(page_tree_store::find_iter_by_id(
                &self.tree_store,
                parent_id,
            ))),
            None => None,
        };
        self.entries.borrow_mut().push(Entry {
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
            },
        });
        let position = {
            let end_index = self.tree_store.iter_n_children(parent_iter.as_ref()) as u32;
            let mut position = match position {
                InsertPosition::Start => 0,
                InsertPosition::End => end_index,
                InsertPosition::Before(id) =>
                    page_tree_store::find_position(&self.tree_store, id, parent)
                        .unwrap_or(end_index),
                InsertPosition::After(id) =>
                    page_tree_store::find_position(&self.tree_store, id, parent)
                        .map(|pos| pos + 1)
                        .unwrap_or(end_index),
            };
            if parent_iter.is_none() {
                let pin_count = self.pinned.borrow().len() as u32;
                position = cmp::max(position, pin_count);
            }
            position
        };
        let title = title.unwrap_or_else(|| String::new());
        page_tree_store::insert(
            &self.tree_store,
            parent_iter.clone(),
            Some(position),
            page_tree_store::Entry {
                id,
                title: title.clone(),
                child_info: "".into(),
                has_children: false,
                child_count: 0,
                style: pango::Style::Italic,
                weight: TITLE_WEIGHT_DEFAULT,
                is_pinned: false,
            },
        );
        self.session_update(|session| session.update_create(id, uri, parent, position));
        if let Some(ref iter) = parent_iter {
            self.recalc(iter);
        }
        Some(id)
    }

    fn recalc(&self, iter: &gtk::TreeIter) {
        recalc(&self.tree_store, iter);
    }

    pub fn contains(&self, id: Id) -> bool {
        self.map_entry(id, |_| ()).is_some()
    }
}

pub enum InsertPosition {
    Start,
    Before(Id),
    After(Id),
    End,
}

pub struct InsertData {
    pub uri: String,
    pub title: Option<String>,
    pub parent: Option<Id>,
    pub position: InsertPosition,
}

struct Entry {
    id: Id,
    uri: String,
    title: Option<String>,
    view: Option<webkit2gtk::WebView>,
    load_state: LoadState,
    favicon: Option<cairo::Surface>,
    is_pinned: bool,
    is_noclose: bool,
}
