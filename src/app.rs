
use std::rc;
use std::cell;
use std::sync;

use gtk;
use webkit2gtk;

use app_action;
use bookmarks;
use domain_settings;
use history;
use main_paned;
use navigation_bar;
use page_bar;
use page_context_menu;
use page_state;
use page_store;
use page_tree_view;
use profile;
use session;
use shortcuts;
use status_bar;
use stored;
use webview;
use window;

fn arg_extract_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if args.contains(&flag.into()) {
        args.retain(|arg| arg != flag);
        true
    } else {
        false
    }
}

fn arg_extract_value(args: &mut Vec<String>, name: &str)
-> Result<Option<String>, ArgumentError> {
    for index in 0..args.len() {
        if &args[index] == name {
            let param = args.remove(index);
            if args.get(index).is_none() {
                return Err(ArgumentError::MissingValue(param));
            }
            let value = args.remove(index);
            if args.contains(&name.into()) {
                return Err(ArgumentError::UnclearProfileParameters);
            }
            return Ok(Some(value));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone)]
pub enum ArgumentError {
    MissingValue(String),
    UnclearProfileParameters,
    MissingProfileParameter,
}

#[derive(Debug, Clone)]
pub struct Arguments {
    is_private: bool,
    profile_mode: profile::Mode,
}

impl Arguments {

    pub fn extract(args: &mut Vec<String>) -> Result<Arguments, ArgumentError> {
        let is_private = arg_extract_flag(args, "--private");
        let profile_mode = {
            let profile_custom = arg_extract_value(args, "--profile-custom")?;
            let profile_xdg = arg_extract_flag(args, "--profile-xdg");
            let profile_local = arg_extract_flag(args, "--profile-local");
            match (profile_custom, profile_xdg, profile_local) {
                (Some(root), false, false) => profile::Mode::Custom(root),
                (None, true, false) => profile::Mode::Xdg,
                (None, false, true) => profile::Mode::Local,
                (None, false, false) => return Err(ArgumentError::MissingProfileParameter),
                _ => return Err(ArgumentError::UnclearProfileParameters),
            }
        };
        Ok(Arguments {
            is_private,
            profile_mode,
        })
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
    shortcuts: rc::Rc<shortcuts::Shortcuts>,
    bookmarks: rc::Rc<bookmarks::Bookmarks>,
    domain_settings: rc::Rc<domain_settings::Settings>,
    is_private: bool,
    #[allow(unused)] page_state_server: page_state::Server,
    page_state_store: sync::Arc<sync::Mutex<page_state::Store>>,
}

pub struct Application {
    data: rc::Rc<Data>,
}

impl Application {

    pub fn new(app: &gtk::Application, app_args: &Arguments) -> Application {

        let profile = profile::Profile::new(&app_args.profile_mode);
        log_trace!("profile {:#?}", profile);

        let history = history::History::open_or_create(
            profile.history(),
            if app_args.is_private {
                history::Mode::Read
            } else {
                history::Mode::ReadWrite
            },
        ).expect("history storage access in application setup");

        let domains = domain_settings::Settings::open_or_create(profile.domain_settings())
            .expect("domain settings storage access in application setup");
        let shortcuts = shortcuts::Shortcuts::open_or_create(profile.shortcuts())
            .expect("shortcuts storage access in application setup");
        let bookmarks = bookmarks::Bookmarks::open_or_create(profile.bookmarks())
            .expect("bookmarks storage access in application setup");

        let (page_store, last_selected) =
            if app_args.is_private {
                (page_store::Store::new_stateless(), None)
            } else {
                let session = session::Session::open_or_create(profile.session())
                    .expect("session storage access in application setup");
                page_store::Store::new_stateful(session)
            };

        let (page_state_server, page_state_store) = page_state::run_server();

        let count = page_store.get_count();
        log_trace!("page store count is {}", count);

        let app = Application {
            data: rc::Rc::new(Data {
                application: app.clone(),
                window: window::create(app),
                main_paned: main_paned::create(),
                page_tree_view: rc::Rc::new(page_tree_view::Map::new()),
                navigation_bar: rc::Rc::new(navigation_bar::Map::new()),
                view_space: gtk::Box::new(gtk::Orientation::Horizontal, 0),
                web_context: webview::create_web_context(
                    app_args.is_private,
                    page_state::InitArguments {
                        instance: page_state_server.name().into(),
                        domain_settings_path: profile.domain_settings().into(),
                    },
                ),
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
                shortcuts: rc::Rc::new(shortcuts),
                bookmarks: rc::Rc::new(bookmarks),
                domain_settings: rc::Rc::new(domains),
                is_private: app_args.is_private,
                page_state_server,
                page_state_store,
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
        shortcuts::setup(&app_handle);
        history::setup(&app_handle);
        bookmarks::setup(&app_handle);
        stored::setup(&app_handle);

        if count == 0 {
            app_handle.page_store()
                .insert(
                    page_store::InsertData::new("https://google.com/".into())
                        .with_title(Some("Google".into()))
                )
                .expect("successful insert into page store during application setup");
        }

        log_debug!("previously selected page: {:?}", last_selected);
        if let Some(id) = last_selected {
            app_handle.page_tree_view().select(id);
        } else {
            app_handle.page_tree_view().select_first();
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

macro_rules! fn_get_gobject_expected {
    ($name:ident: $ty:ty) => {
        pub fn $name(&self) -> $ty {
            match self.data.upgrade().map(|data| data.$name.clone()) {
                Some(value) => value,
                None => panic!(
                    "Expected gobject-based application component '{}' to be available",
                    stringify!($name),
                ),
            }
        }
    }
}

macro_rules! fn_get_arc_mutex_expected {
    ($name:ident: $ty:ty) => {
        pub fn $name(&self) -> sync::Arc<sync::Mutex<$ty>> {
            match self.data.upgrade().map(|data| data.$name.clone()) {
                Some(value) => value,
                None => panic!(
                    "Expected thread-shared application component '{}' to be available",
                    stringify!($name),
                ),
            }
        }
    }
}

macro_rules! fn_get_rc_expected {
    ($name:ident: $ty:ty) => {
        pub fn $name(&self) -> rc::Rc<$ty> {
            match self.data.upgrade().map(|data| data.$name.clone()) {
                Some(value) => value,
                None => panic!(
                    "Expected thread-contained application component '{}' to be available",
                    stringify!($name),
                ),
            }
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

    fn_get_gobject_expected!(application: gtk::Application);
    fn_get_gobject_expected!(window: gtk::ApplicationWindow);
    fn_get_gobject_expected!(view_space: gtk::Box);
    fn_get_gobject_expected!(main_paned: gtk::Paned);
    fn_get_gobject_expected!(bar_size_group: gtk::SizeGroup);
    fn_get_gobject_expected!(web_context: webkit2gtk::WebContext);
    fn_get_gobject_expected!(user_content_manager: webkit2gtk::UserContentManager);

    fn_get_rc_expected!(app_actions: app_action::Map);
    fn_get_rc_expected!(page_context_menu: page_context_menu::Map);
    fn_get_rc_expected!(stored: stored::Map);
    fn_get_rc_expected!(page_tree_view: page_tree_view::Map);
    fn_get_rc_expected!(page_store: page_store::Store);
    fn_get_rc_expected!(status_bar: status_bar::Map);
    fn_get_rc_expected!(navigation_bar: navigation_bar::Map);
    fn_get_rc_expected!(page_bar: page_bar::Map);
    fn_get_rc_expected!(history: history::History);
    fn_get_rc_expected!(shortcuts: shortcuts::Shortcuts);
    fn_get_rc_expected!(bookmarks: bookmarks::Bookmarks);
    fn_get_rc_expected!(domain_settings: domain_settings::Settings);

    fn_get_arc_mutex_expected!(page_state_store: page_state::Store);

    pub fn is_private(&self) -> bool {
        self.data.upgrade().map(|data| data.is_private).unwrap_or(true)
    }

    pub fn get_active(&self) -> Option<page_store::Id> {
        let data = self.data.upgrade()?;
        data.active_page_store_id.get()
    }

    pub fn set_active(&self, id: page_store::Id, view: webkit2gtk::WebView) {
        let data = unwrap_or_return!(self.data.upgrade());
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

    pub fn page_tree_view_widget(&self) -> gtk::TreeView {
        self.page_tree_view().widget().clone()
    }

    pub fn page_tree_store(&self) -> gtk::TreeStore {
        self.page_store().tree_store().clone()
    }
}
