
use std::rc;
use std::cell;

use gtk;
use webkit2gtk;
use cairo;

use webview;
use app;
use page_tree_store;

pub type Id = u32;

pub fn create() -> rc::Rc<Store> {
    rc::Rc::new(Store::new())
}

#[derive(Debug, Clone, Copy)]
pub struct LoadState {
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_loading: bool,
}

pub struct Store {
    last_id: cell::Cell<Id>,
    entries: rc::Rc<cell::RefCell<Vec<Entry>>>,
    tree_store: gtk::TreeStore,
}

impl Store {

    pub fn new() -> Store {
        Store {
            last_id: cell::Cell::new(0),
            entries: rc::Rc::new(cell::RefCell::new(Vec::new())),
            tree_store: page_tree_store::create(),
        }
    }

    pub fn tree_store(&self) -> &gtk::TreeStore { &self.tree_store }

    pub fn get_count(&self) -> usize { self.entries.borrow().len() }

    pub fn get_parent(&self, id: Id) -> Option<Id> {
        use gtk::{ TreeModelExt, Cast };

        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));
        let parent_iter = try_extract!(self.tree_store.iter_parent(&iter));
        let model = self.tree_store.clone().upcast();
        Some(page_tree_store::get::id(&model, &parent_iter))
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
        self.map_entry_mut(id, |entry| entry.uri = value);
        self.update_tree_title(id);
    }

    pub fn set_title(&self, id: Id, value: Option<String>) {
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
        Some(ret_view)
    }

    fn find_next_id(&self) -> Id {
        let mut id = self.last_id.get();
        loop {
            id = id.wrapping_add(1);
            if self.contains(id) {
                continue;
            }
            self.last_id.set(id);
            return id;
        }
    }

    pub fn insert(&self, data: InsertData) -> Option<Id> {
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
            uri: data.uri,
            title: data.title.clone(),
            view: None,
            favicon: None,
            load_state: LoadState {
                can_go_back: false,
                can_go_forward: false,
                is_loading: false,
            },
        });
        let position = match data.position {
            InsertPosition::Start => Some(0),
            InsertPosition::End => None,
            InsertPosition::Before(id) =>
                page_tree_store::find_position(&self.tree_store, id, data.parent),
            InsertPosition::After(id) =>
                page_tree_store::find_position(&self.tree_store, id, data.parent)
                    .map(|pos| pos + 1),
        };
        page_tree_store::insert(
            &self.tree_store,
            parent_iter,
            position,
            page_tree_store::Entry {
                id,
                title: data.title.unwrap_or_else(|| String::new()),
            },
        );
        Some(id)
    }

    pub fn add(&self, uri: String, title: Option<String>) -> Id {
        let mut id = self.last_id.get();
        loop {
            id = id.wrapping_add(1);
            if self.contains(id) {
                continue;
            }
            let view = None;
            self.entries.borrow_mut().push(Entry {
                id,
                uri,
                title,
                view,
                favicon: None,
                load_state: LoadState {
                    can_go_back: false,
                    can_go_forward: false,
                    is_loading: false,
                },
            });
            self.last_id.set(id);
            return id;
        }
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
}
