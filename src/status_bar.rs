
use std::rc;
use std::cell;

use gtk;

use app;

pub struct Bar {
    pub size_group: gtk::SizeGroup,
    pub page_tree_status: gtk::Box,
    pub webview_status: gtk::Box,
    pub page_counter: gtk::Label,
    pub webview_info: gtk::Label,
    pub hover_uri: cell::RefCell<Option<String>>,
}

pub struct Handle {
    bar: rc::Rc<Bar>,
}

impl Handle {

    pub fn new(bar: rc::Rc<Bar>) -> Handle {
        Handle { bar }
    }

    pub fn page_tree_status(&self) -> gtk::Box { self.bar.page_tree_status.clone() }

    pub fn webview_status(&self) -> gtk::Box { self.bar.webview_status.clone() }

    pub fn size_group(&self) -> gtk::SizeGroup { self.bar.size_group.clone() }
    
    pub fn page_counter(&self) -> gtk::Label { self.bar.page_counter.clone() }
    
    pub fn webview_info(&self) -> gtk::Label { self.bar.webview_info.clone() }

    pub fn set_hover_uri(&self, uri: Option<String>) {
        *self.bar.hover_uri.borrow_mut() = uri;
        self.update();
    }

    fn update(&self) {
        use gtk::{ LabelExt };

        match *self.bar.hover_uri.borrow() {
            Some(ref uri) => self.bar.webview_info.set_text(&uri),
            None => self.bar.webview_info.set_text(""),
        }
    }
}

impl Bar {

    pub fn new() -> Bar {
        Bar {
            size_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical),
            page_tree_status: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            webview_status: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            page_counter: gtk::Label::new("0"),
            webview_info: gtk::Label::new(None),
            hover_uri: cell::RefCell::new(None),
        }
    }
}

pub fn setup(app: app::Handle) {
    use gtk::{ SizeGroupExt, BoxExt, LabelExt, WidgetExt };
    use pango;

    let bar = try_extract!(app.status_bar());

    bar.page_tree_status().pack_start(&bar.page_counter(), true, true, 0);

    bar.webview_status().pack_start(&bar.webview_info(), true, true, 0);

    bar.webview_info().set_halign(gtk::Align::Start);
    bar.webview_info().set_ellipsize(pango::EllipsizeMode::End);

    bar.size_group().add_widget(&bar.page_tree_status());
    bar.size_group().add_widget(&bar.webview_status());
}
