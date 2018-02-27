
use std::rc;
use std::cell;

use gtk;
use webkit2gtk;

use navigation_bar;
use page_store;
use app_action;
use status_bar;
use page_bar;
use page_context_menu;
use page_tree_view;
use stored;
use main_paned;
use webview;
use window;
use history;
use session;
use domain_settings;
use page_state;

#[derive(Debug, Clone)]
pub struct Arguments {
    is_private: bool,
}

impl Arguments {

    pub fn extract(args: &mut Vec<String>) -> Arguments {
        let mut is_private = false;
        let mut rest = args.clone();
        let mut ignored = Vec::new();
        while !rest.is_empty() {
            let arg = rest.remove(0);
            if &arg == "--private" {
                is_private = true;
            } else {
                ignored.push(arg);
            }
        }
        *args = ignored;
        Arguments {
            is_private,
        }
    }
}

struct Data {
    application: gtk::Application,
    window: gtk::ApplicationWindow,
    main_paned: gtk::Paned,
    page_tree_view: rc::Rc<page_tree_view::Map>,
    navigation_bar: rc::Rc<navigation_bar::Map>,
    page_store: rc::Rc<page_store::Store>,
    view_space: gtk::Box,
    web_context: webkit2gtk::WebContext,
    user_content_manager: webkit2gtk::UserContentManager,
    active_page_store_id: rc::Rc<cell::Cell<Option<page_store::Id>>>,
    active_webview: rc::Rc<cell::RefCell<Option<webkit2gtk::WebView>>>,
    app_actions: rc::Rc<app_action::Map>,
    status_bar: rc::Rc<status_bar::Map>,
    page_bar: rc::Rc<page_bar::Map>,
    bar_size_group: gtk::SizeGroup,
    select_ignore: cell::Cell<bool>,
    page_context_menu: rc::Rc<page_context_menu::Map>,
    page_tree_target: cell::Cell<Option<page_store::Id>>,
    cached_nav_menu: cell::RefCell<Option<gtk::Menu>>,
    cached_domain_menu: cell::RefCell<Option<gtk::Menu>>,
    stored: rc::Rc<stored::Map>,
    history: rc::Rc<history::History>,
    domain_settings: rc::Rc<domain_settings::Settings>,
    page_state: rc::Rc<page_state::State>,
    is_private: bool,
}

pub struct Application {
    data: rc::Rc<Data>,
}

impl Application {

    pub fn new(app: &gtk::Application, app_args: &Arguments) -> Application {

        let history = history::History::open_or_create(
            "_profile/config/history.db",
            if app_args.is_private {
                history::Mode::Read
            } else {
                history::Mode::ReadWrite
            },
        ).expect("history storage access");
        let domains = domain_settings::Settings::open_or_create(
            "_profile/config/domain_settings.db",
        ).expect("domain settings storage access");
        let page_state = page_state::State::open_or_create(
            "_profile/runtime/page_state.db",
        ).expect("page state inter-process storage access");
        page_state.clear();

        let (page_store, last_selected) =
            if app_args.is_private {
                (page_store::Store::new_stateless(), None)
            } else {
                let session = session::Session::open_or_create("_profile/config/session.db")
                    .expect("session storage access");
                page_store::Store::new_stateful(session)
            };

        let count = page_store.get_count();

        let app = Application {
            data: rc::Rc::new(Data {
                application: app.clone(),
                window: window::create(app),
                main_paned: main_paned::create(),
                page_tree_view: rc::Rc::new(page_tree_view::Map::new()),
                navigation_bar: rc::Rc::new(navigation_bar::Map::new()),
                view_space: gtk::Box::new(gtk::Orientation::Horizontal, 0),
                web_context: webview::create_web_context(app_args.is_private),
                user_content_manager: webview::create_user_content_manager(),
                page_store: rc::Rc::new(page_store),
                active_page_store_id: rc::Rc::new(cell::Cell::new(None)),
                active_webview: rc::Rc::new(cell::RefCell::new(None)),
                app_actions: rc::Rc::new(app_action::create()),
                status_bar: rc::Rc::new(status_bar::Map::new()),
                page_bar: rc::Rc::new(page_bar::Map::new()),
                bar_size_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical),
                select_ignore: cell::Cell::new(false),
                page_context_menu: rc::Rc::new(page_context_menu::create()),
                page_tree_target: cell::Cell::new(None),
                cached_nav_menu: cell::RefCell::new(None),
                cached_domain_menu: cell::RefCell::new(None),
                stored: rc::Rc::new(stored::Map::new()),
                history: rc::Rc::new(history),
                domain_settings: rc::Rc::new(domains),
                page_state: rc::Rc::new(page_state),
                is_private: app_args.is_private,
            }),
        };

        let app_handle = app.handle();

        log_debug!("component setup");
        window::setup(&app_handle);
        main_paned::setup(&app_handle);
        page_tree_view::setup(&app_handle);
        navigation_bar::setup(&app_handle);
        app_action::setup(&app_handle);
        status_bar::setup(&app_handle);
        page_bar::setup(&app_handle);
        page_context_menu::setup(&app_handle);
        page_store::setup(&app_handle);
        webview::setup(&app_handle);
        history::setup(&app_handle);
        stored::setup(&app_handle);

        if count == 0 {
            app_handle.page_store().expect("page store during init").insert(
                page_store::InsertData::new("https://google.com/".into())
                    .with_title(Some("Google".into()))
            ).expect("successful insert into page store during init");
        }

        log_debug!("previously selected page: {:?}", last_selected);
        if let Some(id) = last_selected {
            app_handle.page_tree_view().expect("page tree view during init").select(id);
        } else {
            app_handle.page_tree_view().expect("page tree view during init").select_first();
        }

        app
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

macro_rules! fn_get_gobject {
    ($name:ident: $ty:ty) => {
        pub fn $name(&self) -> Option<$ty> {
            self.data.upgrade().map(|data| data.$name.clone())
        }
    }
}

macro_rules! fn_get_rc {
    ($name:ident: $ty:ty) => {
        pub fn $name(&self) -> Option<rc::Rc<$ty>> {
            self.data.upgrade().map(|data| data.$name.clone())
        }
    }
}

macro_rules! fn_set_cached {
    ($field:ident: $setter:ident $ty:ty) => {
        pub fn $setter(&self, value: Option<$ty>) {
            self.data.upgrade().map(|data| *data.$field.borrow_mut() = value);
        }
    }
}

macro_rules! fn_set_cell {
    ($field:ident: $setter:ident $ty:ty) => {
        pub fn $setter(&self, value: $ty) {
            self.data.upgrade().map(|data| data.$field.set(value));
        }
    }
}

macro_rules! fn_get_cell_flatten {
    ($field:ident: $getter:ident $ty:ty) => {
        pub fn $getter(&self) -> Option<$ty> {
            self.data.upgrade().and_then(|data| data.$field.get())
        }
    }
}

macro_rules! fn_get_cell_default {
    ($field:ident: $getter:ident $ty:ty, $default:expr) => {
        pub fn $getter(&self) -> $ty {
            self.data.upgrade().map(|data| data.$field.get()).unwrap_or_else(|| $default)
        }
    }
}

impl Handle {

    fn_set_cached!(cached_nav_menu: set_cached_nav_menu gtk::Menu);
    fn_set_cached!(cached_domain_menu: set_cached_domain_menu gtk::Menu);

    fn_set_cell!(page_tree_target: set_page_tree_target Option<page_store::Id>);
    fn_get_cell_flatten!(page_tree_target: get_page_tree_target page_store::Id);

    fn_set_cell!(select_ignore: set_select_ignored bool);
    fn_get_cell_default!(select_ignore: is_select_ignored bool, false);

    fn_get_gobject!(application: gtk::Application);
    fn_get_gobject!(window: gtk::ApplicationWindow);
    fn_get_gobject!(view_space: gtk::Box);
    fn_get_gobject!(main_paned: gtk::Paned);
    fn_get_gobject!(bar_size_group: gtk::SizeGroup);
    fn_get_gobject!(web_context: webkit2gtk::WebContext);
    fn_get_gobject!(user_content_manager: webkit2gtk::UserContentManager);

    fn_get_rc!(app_actions: app_action::Map);
    fn_get_rc!(page_context_menu: page_context_menu::Map);
    fn_get_rc!(stored: stored::Map);
    fn_get_rc!(page_tree_view: page_tree_view::Map);
    fn_get_rc!(page_store: page_store::Store);
    fn_get_rc!(status_bar: status_bar::Map);
    fn_get_rc!(navigation_bar: navigation_bar::Map);
    fn_get_rc!(page_bar: page_bar::Map);
    fn_get_rc!(history: history::History);
    fn_get_rc!(domain_settings: domain_settings::Settings);
    fn_get_rc!(page_state: page_state::State);

    pub fn is_private(&self) -> bool {
        self.data.upgrade().map(|data| data.is_private).unwrap_or(true)
    }

    pub fn get_active(&self) -> Option<page_store::Id> {
        let data = try_extract!(self.data.upgrade());
        data.active_page_store_id.get()
    }

    pub fn set_active(&self, id: page_store::Id, view: webkit2gtk::WebView) {
        let data = try_extract!(self.data.upgrade());
        data.active_page_store_id.set(Some(id));
        *data.active_webview.borrow_mut() = Some(view);
    }

    pub fn active_webview(&self) -> Option<webkit2gtk::WebView> {
        self.data.upgrade().and_then(|data| match *data.active_webview.borrow() {
            Some(ref view) => Some(view.clone()),
            None => None,
        })
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

    pub fn without_select<F, R>(&self, body: F) -> R where F: FnOnce() -> R {
        self.set_select_ignored(true);
        let result = body();
        self.set_select_ignored(false);
        result
    }

    pub fn page_tree_view_widget(&self) -> Option<gtk::TreeView> {
        self.page_tree_view().map(|map| map.widget().clone())
    }

    pub fn page_tree_store(&self) -> Option<gtk::TreeStore> {
        self.data.upgrade().map(|data| data.page_store.tree_store().clone())
    }
}
