
extern crate gtk;
extern crate gio;
extern crate gdk;
extern crate glib;
extern crate cairo;
extern crate pango;
extern crate webkit2gtk;
extern crate rusqlite;

#[macro_use] mod macros;

pub mod app;
pub mod window;
pub mod main_paned;
pub mod scrolled;
pub mod page_tree_view;
pub mod navigation_bar;
pub mod page_store;
pub mod action;
pub mod webview;
pub mod app_action;
pub mod status_bar;
pub mod mouse;
pub mod page_bar;
pub mod bar;
pub mod menu;
pub mod page_context_menu;
pub mod session;
pub mod recently_closed;
pub mod signal;
pub mod text;
#[path="page_tree_store.rs"] pub mod new_page_tree_store;

use std::rc;
use std::cell;
use std::sync;

const LOG_OFF: usize = 0;
const LOG_DEBUG: usize = 1;
const LOG_TRACE: usize = 2;

static CURRENT_LOG_LEVEL: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(LOG_OFF);


mod_tree_store! {
    page_tree_store:
    struct {
        id: ::page_store::Id,
        title: String,
        child_info: String,
        has_children: bool,
        child_count: u32,
        style: ::pango::Style,
        weight: i32,
        is_pinned: bool,
    }
    pub fn set_title(store: &gtk::TreeStore, iter: &gtk::TreeIter, title: &str) {
        use ::gtk::{ TreeStoreExtManual, ToValue };
        store.set_value(iter, self::index::title, &title.to_value());
    }
    pub fn find_position(
        store: &gtk::TreeStore,
        id: ::page_store::Id,
        parent_id: Option<::page_store::Id>,
    ) -> Option<u32> {
        use gtk::{ TreeModelExt, Cast };

        let parent_iter = match parent_id {
            Some(parent_id) => Some(try_extract!(find_iter_by_id(store, parent_id))),
            None => None,
        };
        let count = store.iter_n_children(parent_iter.as_ref());
        let model = store.clone().upcast();
        for index in 0..count {
            match store.iter_nth_child(parent_iter.as_ref(), index) {
                Some(iter) => {
                    let iter_id: ::page_store::Id = self::get::id(&model, &iter);
                    if iter_id == id {
                        return Some(index as u32);
                    }
                },
                None => (),
            }
        }
        None
    }
    pub fn find_iter_by_id(
        store: &gtk::TreeStore,
        id: ::page_store::Id,
    ) -> Option<gtk::TreeIter> {
        use gtk::{ Cast };

        fn find_in_children(
            store: &gtk::TreeModel,
            id: ::page_store::Id,
            parent: Option<&gtk::TreeIter>,
        ) -> Option<gtk::TreeIter> {
            use gtk::{ TreeModelExt };

            let count = store.iter_n_children(parent);
            for index in 0..count {
                match store.iter_nth_child(parent, index) {
                    Some(iter) => {
                        let iter_id: ::page_store::Id = self::get::id(store, &iter);
                        if iter_id == id {
                            return Some(iter);
                        }
                        if let Some(iter) = find_in_children(store, id, Some(&iter)) {
                            return Some(iter);
                        }
                    },
                    None => (),
                }
            }
            None
        }

        let store = store.clone().upcast::<gtk::TreeModel>();
        find_in_children(&store, id, None)
    }
}


fn main() {
    use std::env;
    use gio;
    use gio::{ ApplicationExt, ApplicationExtManual };

    let log_level = match env::var("BRIMSTONE_LOG") {
        Ok(value) => match value.as_str() {
            "debug" => LOG_DEBUG,
            "trace" => LOG_TRACE,
            _ => LOG_OFF,
        },
        _ => LOG_OFF,
    };
    CURRENT_LOG_LEVEL.store(log_level, sync::atomic::Ordering::SeqCst);

    log_debug!("construct application");
    let app = gtk::Application::new(
        "web.brimstone",
        gio::ApplicationFlags::empty(),
    ).expect("Gtk initialization failed");

    let app_space = rc::Rc::new(cell::RefCell::new(None));
    let app_space_sink = app_space.clone();

    app.connect_startup(move |app| setup(app, &app_space_sink));
    app.connect_activate(|_| ());

    log_debug!("run application");
    let args = env::args().collect::<Vec<_>>();
    app.run(&args);
    log_debug!("run complete");
}

fn setup(app: &gtk::Application, app_space: &rc::Rc<cell::RefCell<Option<app::Application>>>) {

    log_debug!("loading session");
    let session_storage = session::Storage::open_or_create("_profile/config/session.db").unwrap();

    log_debug!("assembling components");
    let page_store = page_store::Store::new_stateful(session_storage);
    let count = page_store.get_count();

    let app = app::Application::new(app::Data {
        application: app.clone(),
        window: window::create(app),
        main_paned: main_paned::create(),
        page_tree_view: page_tree_view::create(),
        navigation_bar: rc::Rc::new(navigation_bar::create()),
        view_space: gtk::Box::new(gtk::Orientation::Horizontal, 0),
        web_context: webview::create_web_context(),
        user_content_manager: webview::create_user_content_manager(),
        page_store: rc::Rc::new(page_store),
        active_page_store_id: rc::Rc::new(cell::Cell::new(None)),
        active_webview: rc::Rc::new(cell::RefCell::new(None)),
        app_actions: rc::Rc::new(app_action::create()),
        empty_favicon: cairo::ImageSurface::create(cairo::Format::A8, 16, 16).unwrap(),
        status_bar: rc::Rc::new(status_bar::Bar::new()),
        page_bar: rc::Rc::new(page_bar::create()),
        bar_size_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical),
        select_ignore: cell::Cell::new(false),
        page_context_menu: rc::Rc::new(page_context_menu::create()),
        page_tree_target: cell::Cell::new(None),
        cached_nav_menu: cell::RefCell::new(None),
    });

    log_debug!("component setup");
    window::setup(app.handle());
    main_paned::setup(app.handle());
    page_tree_view::setup(app.handle());
    navigation_bar::setup(app.handle());
    app_action::setup(app.handle());
    status_bar::setup(app.handle());
    page_bar::setup(app.handle());
    page_context_menu::setup(app.handle());
    page_store::setup(app.handle());

    app.handle().perform(action::page::UpdateCounter);

    window::present(&app);

    if count == 0 {
        let prev = app.handle().perform(::action::page::Create {
            title: Some("Test Crates".into()),
            uri: "https://crates.io".into(),
            parent: None,
            position: page_store::InsertPosition::Start,
        }).unwrap();
        let _prev = app.handle().perform(::action::page::Create {
            title: Some("Test Rust".into()),
            uri: "https://www.rust-lang.org".into(),
            parent: Some(prev),
            position: page_store::InsertPosition::After(prev),
        }).unwrap();
        app.handle().perform(::action::page::Create {
            title: Some("Test Youtube".into()),
            uri: "https://www.youtube.com".into(),
            parent: None,
            position: page_store::InsertPosition::End,
        });
    }

    *app_space.borrow_mut() = Some(app);
}
