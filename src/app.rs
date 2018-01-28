
use std::rc;
use std::cell;

use gtk;
use webkit2gtk;
use cairo;

use navigation_bar;
use page_store;
use app_action;
use status_bar;
use page_bar;

pub struct Data {
    pub application: gtk::Application,
    pub window: gtk::ApplicationWindow,
    pub main_paned: gtk::Paned,
    pub page_tree_view: gtk::TreeView,
    pub navigation_bar: rc::Rc<navigation_bar::Bar>,
    pub page_store: rc::Rc<page_store::Store>,
    pub view_space: gtk::Box,
    pub web_context: webkit2gtk::WebContext,
    pub user_content_manager: webkit2gtk::UserContentManager,
    pub active_page_store_id: rc::Rc<cell::Cell<Option<page_store::Id>>>,
    pub active_webview: rc::Rc<cell::RefCell<Option<webkit2gtk::WebView>>>,
    pub app_actions: rc::Rc<app_action::Map>,
    pub empty_favicon: cairo::ImageSurface,
    pub status_bar: rc::Rc<status_bar::Bar>,
    pub page_bar: rc::Rc<page_bar::Bar>,
    pub bar_size_group: gtk::SizeGroup,
    pub select_ignore: cell::Cell<bool>,
}

impl Data {

    pub fn is_active(&self, id: page_store::Id) -> bool {
        if let Some(active_id) = self.active_page_store_id.get() {
            if active_id == id {
                return true;
            }
        }
        false
    }
}

pub struct Application {
    data: rc::Rc<Data>,
}

impl Application {

    pub fn new(data: Data) -> Application {
        Application {
            data: rc::Rc::new(data),
        }
    }

    pub fn handle(&self) -> Handle {
        Handle {
            data: rc::Rc::downgrade(&self.data),
        }
    }
}

#[derive(Clone)]
pub struct Handle {
    data: rc::Weak<Data>,
}

impl Handle {

    pub fn get_active(&self) -> Option<page_store::Id> {
        let data = try_extract!(self.data.upgrade());
        data.active_page_store_id.get()
    }

    pub fn set_active(&self, id: page_store::Id, view: webkit2gtk::WebView) {
        let data = try_extract!(self.data.upgrade());
        data.active_page_store_id.set(Some(id));
        *data.active_webview.borrow_mut() = Some(view);
    }

    pub fn is_active(&self, id: page_store::Id) -> bool {
        let data = match self.data.upgrade() {
            Some(data) => data,
            None => return false,
        };
        if let Some(active_id) = data.active_page_store_id.get() {
            return active_id == id;
        }
        false
    }

    pub fn is_select_ignored(&self) -> bool {
        self.data.upgrade().map(|data| data.select_ignore.get()).unwrap_or(true)
    }

    pub fn set_select_ignored(&self, ignored: bool) {
        self.data.upgrade().map(|data| data.select_ignore.set(ignored));
    }

    pub fn without_select<F, R>(&self, body: F) -> R where F: FnOnce() -> R {
        self.data.upgrade().map(|data| data.select_ignore.set(true));
        let result = body();
        self.data.upgrade().map(|data| data.select_ignore.set(false));
        result
    }

    pub fn empty_favicon(&self) -> Option<cairo::ImageSurface> {
        self.data.upgrade().map(|data| data.empty_favicon.clone())
    }

    pub fn application(&self) -> Option<gtk::Application> {
        self.data.upgrade().map(|data| data.application.clone())
    }

    pub fn app_actions(&self) -> Option<rc::Rc<app_action::Map>> {
        self.data.upgrade().map(|data| data.app_actions.clone())
    }

    pub fn active_webview(&self) -> Option<webkit2gtk::WebView> {
        self.data.upgrade().and_then(|data| match *data.active_webview.borrow() {
            Some(ref view) => Some(view.clone()),
            None => None,
        })
    }

    pub fn window(&self) -> Option<gtk::ApplicationWindow> {
        self.data.upgrade().map(|data| data.window.clone())
    }

    pub fn view_space(&self) -> Option<gtk::Box> {
        self.data.upgrade().map(|data| data.view_space.clone())
    }

    pub fn main_paned(&self) -> Option<gtk::Paned> {
        self.data.upgrade().map(|data| data.main_paned.clone())
    }

    pub fn page_tree_view(&self) -> Option<gtk::TreeView> {
        self.data.upgrade().map(|data| data.page_tree_view.clone())
    }

    pub fn bar_size_group(&self) -> Option<gtk::SizeGroup> {
        self.data.upgrade().map(|data| data.bar_size_group.clone())
    }

    pub fn page_tree_store(&self) -> Option<gtk::TreeStore> {
        self.data.upgrade().map(|data| data.page_store.tree_store().clone())
    }

    pub fn page_bar(&self) -> Option<page_bar::Handle> {
        self.data.upgrade().map(|data| page_bar::Handle::new(data.page_bar.clone()))
    }

    pub fn status_bar(&self) -> Option<status_bar::Handle> {
        self.data.upgrade().map(|data| status_bar::Handle::new(data.status_bar.clone()))
    }

    pub fn navigation_bar(&self) -> Option<navigation_bar::Handle> {
        self.data.upgrade().map(|data| navigation_bar::Handle::new(data.navigation_bar.clone()))
    }

    pub fn page_store(&self) -> Option<rc::Rc<page_store::Store>> {
        self.data.upgrade().map(|data| data.page_store.clone())
    }

    pub fn web_context(&self) -> Option<webkit2gtk::WebContext> {
        self.data.upgrade().map(|data| data.web_context.clone())
    }

    pub fn user_content_manager(&self) -> Option<webkit2gtk::UserContentManager> {
        self.data.upgrade().map(|data| data.user_content_manager.clone())
    }

    pub fn perform<A>(&self, action: A) -> A::Result
    where A: Perform {
        action.perform(self)
    }

    pub fn with_cloned<F, R>(&self, callback: F) -> R
    where F: FnOnce(Handle) -> R {
        callback(self.clone())
    }
}

pub trait Perform {

    type Result;

    fn perform(self, app: &Handle) -> Self::Result;
}
