
extern crate cairo;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate pango;
extern crate rusqlite;
extern crate serde;
extern crate serde_json;
extern crate webkit2gtk;
extern crate xdg;

extern crate brimstone_storage as storage;
extern crate brimstone_domain_settings as domain_settings;
extern crate brimstone_page_state as page_state;

#[macro_use] mod macros;

pub mod app;
pub mod app_action;
pub mod bar;
pub mod bookmarks;
pub mod dynamic;
pub mod history;
pub mod layout;
pub mod main_paned;
pub mod menu;
pub mod mouse;
pub mod navigation_bar;
pub mod page_bar;
pub mod page_context_menu;
pub mod page_store;
pub mod page_tree_store;
pub mod page_tree_view;
pub mod profile;
pub mod recently_closed;
pub mod script_dialog;
pub mod scrolled;
pub mod session;
pub mod shortcuts;
pub mod signal;
pub mod status_bar;
pub mod stored;
pub mod text;
pub mod webview;
pub mod window;

use std::rc;
use std::cell;
use std::sync;

const LOG_OFF: usize = 0;
const LOG_DEBUG: usize = 1;
const LOG_TRACE: usize = 2;

static CURRENT_LOG_LEVEL: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(LOG_OFF);

fn main() {
    use gio::prelude::*;
    use std::env;
    use gio;

    env::set_var("RUST_BACKTRACE", "1");

    let log_level = match env::var("BRIMSTONE_LOG") {
        Ok(value) => match value.as_str() {
            "debug" => LOG_DEBUG,
            "trace" => LOG_TRACE,
            _ => LOG_OFF,
        },
        _ => LOG_OFF,
    };
    CURRENT_LOG_LEVEL.store(log_level, sync::atomic::Ordering::SeqCst);
    
    let mut args = env::args().collect::<Vec<_>>();
    let app_args = match app::Arguments::extract(&mut args) {
        Ok(args) => args,
        Err(err) => {
            match err {
                app::ArgumentError::MissingValue(param) =>
                    eprintln!("Missing value for {} parameter.", param),
                app::ArgumentError::UnclearProfileParameters =>
                    eprintln!("Unclear profile variant selection parameters."),
                app::ArgumentError::MissingProfileParameter =>
                    eprintln!("No profile parameters were specified."),
            }
            return;
        },
    };

    log_debug!("construct application");
    let app = gtk::Application::new("web.brimstone", gio::ApplicationFlags::empty())
        .expect("successfully initialized application");

    let app_space = rc::Rc::new(cell::RefCell::new(None));
    let app_space_sink = app_space.clone();

    app.connect_startup(with_cloned!(app_args, move |app| {
        *app_space_sink.borrow_mut() = Some(setup(app, &app_args));
    }));
    app.connect_activate(|_| ());

    log_debug!("run application");
    app.run(&args);
    drop(app_space);
    log_debug!("run complete");
}

fn setup(app: &gtk::Application, app_args: &app::Arguments) -> app::Application {

    let app = app::Application::new(app, app_args);
    let handle = app.handle();

    window::present(&handle);

    app
}
