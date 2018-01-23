
extern crate gtk;
extern crate gio;
extern crate webkit2gtk;

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

use std::rc;
use std::cell;

mod_tree_store! {
    page_tree_store:
    struct {
        id: ::page_store::Id,
        title: String,
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

    let app = gtk::Application::new(
        "web.brimstone",
        gio::ApplicationFlags::empty(),
    ).expect("Gtk initialization failed");

    let app_space = rc::Rc::new(cell::RefCell::new(None));
    let app_space_sink = app_space.clone();

    app.connect_startup(move |app| setup(app, &app_space_sink));
    app.connect_activate(|_| ());

    let args = env::args().collect::<Vec<_>>();
    app.run(&args);
}

fn setup(app: &gtk::Application, app_space: &rc::Rc<cell::RefCell<Option<app::Application>>>) {
    use webkit2gtk::{ WebContextExt };
    use gtk::{ ToVariant };

    let app = app::Application::new(app::Data {
        application: app.clone(),
        window: window::create(app),
        main_paned: main_paned::create(),
        page_tree_view: page_tree_view::create(),
        page_tree_store: page_tree_store::create(),
        navigation_bar: rc::Rc::new(navigation_bar::create()),
        view_space: gtk::Box::new(gtk::Orientation::Horizontal, 0),
        web_context: webview::create_web_context(),
        user_content_manager: webview::create_user_content_manager(),
        page_store: page_store::create(),
        active_page_store_id: rc::Rc::new(cell::Cell::new(None)),
        active_webview: rc::Rc::new(cell::RefCell::new(None)),
        app_actions: rc::Rc::new(app_action::create()),
    });

    window::setup(app.handle());
    main_paned::setup(app.handle());
    page_tree_view::setup(app.handle());
    navigation_bar::setup(app.handle());
    app_action::setup(app.handle());

    window::present(&app);

    app.handle().perform(::action::page::Create {
        title: Some("Test Crates".into()),
        url: "https://crates.io".into(),
        parent: None,
    });
    app.handle().perform(::action::page::Create {
        title: Some("Test Rust".into()),
        url: "https://www.rust-lang.org".into(),
        parent: None,
    });
    app.handle().perform(::action::page::Create {
        title: Some("Test Youtube".into()),
        url: "https://www.youtube.com".into(),
        parent: None,
    });

    *app_space.borrow_mut() = Some(app);
}
