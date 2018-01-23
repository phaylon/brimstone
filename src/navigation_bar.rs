
use std::rc;

use gtk;

use app;

pub struct Bar {
    pub container: gtk::Box,
    pub address_entry: gtk::Entry,
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
}

pub fn create() -> Bar {
    Bar {
        container: create_container(),
        address_entry: create_address_entry(),
    }
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
    use gtk::{ ContainerExt, BoxExt, EntryExt };
    use webkit2gtk::{ WebViewExt };

    let bar = app.navigation_bar().unwrap().bar;
    bar.container.pack_start(&bar.address_entry, true, true, 0);

    bar.address_entry.connect_activate(with_cloned!(app, move |entry| {
        let url = match entry.get_text() {
            Some(text) => text,
            None => return,
        };
        let webview = match app.active_webview() {
            Some(webview) => webview,
            None => return,
        };
        webview.load_uri(&url);
    }));
}
