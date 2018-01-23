
use std::rc;
use std::cell;

use webkit2gtk;

use webview;
use app;

pub type Id = u32;

pub fn create() -> rc::Rc<Store> {
    rc::Rc::new(Store::new())
}

pub struct Store {
    last_id: cell::Cell<Id>,
    entries: rc::Rc<cell::RefCell<Vec<Entry>>>,
}

impl Store {

    pub fn new() -> Store {
        Store {
            last_id: cell::Cell::new(0),
            entries: rc::Rc::new(cell::RefCell::new(Vec::new())),
        }
    }

    pub fn url_for(&self, id: Id) -> Option<String> {
        self.map_entry(id, |entry| entry.url.clone())
//        self.find_entry(id).map(|entry| entry.url.as_str())
    }

    pub fn title_for(&self, id: Id) -> Option<String> {
        self.map_entry(id, |entry| match entry.title {
            Some(ref title) => title.clone(),
            None => entry.url.clone(),
        })
    }

    pub fn set_url_for(&self, id: Id, value: String) {
        self.map_entry_mut(id, |entry| entry.url = value);
    }

    pub fn set_title_for(&self, id: Id, value: Option<String>) {
        self.map_entry_mut(id, |entry| entry.title = value);
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

    pub fn view_for(&self, id: Id, data: &app::Data) -> Option<webkit2gtk::WebView> {
        use webkit2gtk::{ WebViewExt };

        let view = self.map_entry(id, |entry| entry.view.clone());
        match view {
            Some(Some(view)) => return Some(view),
            Some(None) => (),
            None => return None,
        };

        let url = self.map_entry(id, |entry| entry.url.clone()).unwrap();
        let new_view = webview::create(id, data);
        new_view.load_uri(&url);
        let ret_view = new_view.clone();
        self.map_entry_mut(id, move |entry| entry.view = Some(new_view));
        Some(ret_view)

        /*
        let web_context = data.web_context.clone();
        let user_content_manager = data.user_content_manager.clone();

        let entry = match self.find_entry_mut(id) {
            Some(entry) => entry,
            None => return None,
        };
        match entry.view {
            Some(ref mut view) => return Some(view.clone()),
            None => (),
        }

        let new_view = webview::create(&web_context, &user_content_manager, data);
        new_view.load_uri(&entry.url);
        entry.view = Some(new_view.clone());
        Some(new_view)
        */
    }

    /*
    fn find_entry_mut(&mut self, id: Id) -> Option<&mut Entry> {
        for entry in &mut self.entries {
            if entry.id == id {
                return Some(entry);
            }
        }
        None
    }

    fn find_entry(&self, id: Id) -> Option<&Entry> {
        for entry in &self.entries {
            if entry.id == id {
                return Some(entry);
            }
        }
        None
    }
    */

    pub fn add(&self, url: String, title: Option<String>) -> Id {
        let mut id = self.last_id.get();
        loop {
            id = id.wrapping_add(1);
            if self.contains(id) {
                continue;
            }
            let view = None;
            self.entries.borrow_mut().push(Entry { id, url, title, view });
            self.last_id.set(id);
            return id;
        }
    }

    pub fn contains(&self, id: Id) -> bool {
        self.map_entry(id, |_| ()).is_some()
    }
}

struct Entry {
    id: Id,
    url: String,
    title: Option<String>,
    view: Option<webkit2gtk::WebView>,
}
