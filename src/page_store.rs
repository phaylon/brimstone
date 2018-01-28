
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
        let child_title: String = page_tree_store::get::title(&model, &child_iter);
        Some(child_id)
    }

    pub fn has_children(&self, id: Id) -> Option<i32> {
        use gtk::{ TreeModelExt, Cast };

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

    fn children(&self, parent: Option<&gtk::TreeIter>) -> Vec<(Id, gtk::TreeIter)> {
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
        use gtk::{ Cast, TreeStoreExt, TreeModelExt, ContainerExt, WidgetExt };

        fn deep_close(
            page_store: &Store,
            model: &gtk::TreeModel,
            store: &gtk::TreeStore,
            iter: &gtk::TreeIter,
        ) {
            let id: Id = page_tree_store::get::id(&model, &iter);
            for (child_id, child_iter) in page_store.children(Some(iter)) {
                deep_close(page_store, model, store, &child_iter);
            }
            store.remove(iter);
            let mut entries = page_store.entries.borrow_mut();
            let index = entries.iter().position(|entry| entry.id == id).unwrap();
            let entry = entries.remove(index);
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
                let &(child_id, ref child_iter) = &children[child_index];
                self.move_to(
                    child_id,
                    curr_parent,
                    (curr_position + child_index as u32) as i32,
                );
            }
        }
        deep_close(self, &model, &self.tree_store, &iter);
    }

    pub fn move_to(&self, id: Id, parent: Option<Id>, position: i32) {
        use gtk::{ TreeStoreExt, TreeModelExt, TreeStoreExtManual };
        
        let iter = try_extract!(page_tree_store::find_iter_by_id(&self.tree_store, id));

        let mut values = Vec::new();
        for column in 0..self.tree_store.get_n_columns() {
            values.push(self.tree_store.get_value(&iter, column));
        }

        self.tree_store.remove(&iter);

        let parent_iter = parent
            .map(|id| page_tree_store::find_iter_by_id(&self.tree_store, id).unwrap());

        let iter = self.tree_store.insert(parent_iter.as_ref(), position);

        for column in 0..values.len() {
            self.tree_store.set_value(&iter, column as u32, &values[column]);
        }
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
