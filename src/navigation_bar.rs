
use std::rc;

use gtk;

use app;
use app_action;

pub struct Bar {
    pub container: gtk::Box,
    pub address_entry: gtk::Entry,
    pub go_back_button: gtk::Button,
    pub go_forward_button: gtk::Button,
    pub reload_button: gtk::Button,
    pub stop_button: gtk::Button,
}

pub struct Handle {
    bar: rc::Rc<Bar>,
}

impl Handle {

    pub fn new(bar: rc::Rc<Bar>) -> Handle {
        Handle { bar }
    }

    pub fn container(&self) -> gtk::Box { self.bar.container.clone() }

    pub fn address_entry(&self) -> gtk::Entry { self.bar.address_entry.clone() }

    pub fn go_back_button(&self) -> gtk::Button { self.bar.go_back_button.clone() }

    pub fn go_forward_button(&self) -> gtk::Button { self.bar.go_forward_button.clone() }

    pub fn stop_button(&self) -> gtk::Button { self.bar.stop_button.clone() }

    pub fn reload_button(&self) -> gtk::Button { self.bar.reload_button.clone() }
}

pub fn create() -> Bar {
    Bar {
        container: create_container(),
        address_entry: create_address_entry(),
        go_back_button: create_nav_button("go-previous", false, true),
        go_forward_button: create_nav_button("go-next", false, true),
        reload_button: create_nav_button("view-refresh", true, true),
        stop_button: create_nav_button("process-stop", true, false),
    }
}

fn create_nav_button(name: &str, sensitivity: bool, visibility: bool) -> gtk::Button {
    use gtk::{ WidgetExt };

    let button = gtk::Button::new_from_icon_name(
        name,
        gtk::IconSize::SmallToolbar.into(),
    );
    button.set_sensitive(sensitivity);
    button.set_visible(visibility);
    if !visibility {
        button.set_no_show_all(true);
    }
    button
}

fn create_address_entry() -> gtk::Entry {
    use gtk::{ EntryExt };

    let entry = gtk::Entry::new();
    entry.set_activates_default(false);

    entry
}

fn create_container() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, 5)
}

pub fn setup(app: app::Handle) {
    use gtk::{ BoxExt, EntryExt, WidgetExt, ActionableExt };
    use webkit2gtk::{ WebViewExt };
    use gio::{ ActionExt };
    use gdk;

    let bar = app.navigation_bar().unwrap().bar;

    bar.container.pack_start(&bar.go_back_button, false, true, 0);
    bar.container.pack_start(&bar.go_forward_button, false, true, 0);
    bar.container.pack_start(&bar.address_entry, true, true, 0);
    bar.container.pack_start(&bar.reload_button, false, true, 0);
    bar.container.pack_start(&bar.stop_button, false, true, 0);

    bar.address_entry.connect_activate(with_cloned!(app, move |entry| {
        let uri = try_extract!(entry.get_text());
        let webview = try_extract!(app.active_webview());
        webview.load_uri(&uri);
    }));

    bar.go_back_button.set_action_name(Some(app_action::ACTION_GO_BACK));
    bar.go_forward_button.set_action_name(Some(app_action::ACTION_GO_FORWARD));
    bar.stop_button.set_action_name(Some(app_action::ACTION_STOP));

    bar.reload_button.connect_button_release_event(with_cloned!(app, move |_button, event| {
        (||{
            let app_actions = try_extract!(app.app_actions());
            if event.get_state() == gdk::ModifierType::SHIFT_MASK {
                app_actions.reload_bp_action.activate(None);
            } else {
                app_actions.reload_action.activate(None);
            }
        })();
        gtk::prelude::Inhibit(false)
    }));
}
