
use std::cell;

use gtk;

use app;

pub struct Map {
    pub size_group: gtk::SizeGroup,
    pub page_tree_status: gtk::Box,
    pub webview_status: gtk::Box,
    pub page_counter: gtk::Label,
    pub webview_info: gtk::Label,
    pub hover_uri: cell::RefCell<Option<String>>,
}

impl Map {

    pub fn new() -> Map {
        Map {
            size_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical),
            page_tree_status: gtk::Box::new(gtk::Orientation::Horizontal, 5),
            webview_status: gtk::Box::new(gtk::Orientation::Horizontal, 5),
            page_counter: gtk::Label::new("0"),
            webview_info: gtk::Label::new(None),
            hover_uri: cell::RefCell::new(None),
        }
    }

    pub fn page_tree_status(&self) -> gtk::Box { self.page_tree_status.clone() }

    pub fn webview_status(&self) -> gtk::Box { self.webview_status.clone() }

    pub fn size_group(&self) -> gtk::SizeGroup { self.size_group.clone() }
    
    pub fn page_counter(&self) -> gtk::Label { self.page_counter.clone() }
    
    pub fn webview_info(&self) -> gtk::Label { self.webview_info.clone() }

    pub fn set_hover_uri(&self, uri: Option<String>) {
        *self.hover_uri.borrow_mut() = uri;
        self.update();
    }

    fn update(&self) {
        use gtk::{ LabelExt };

        match *self.hover_uri.borrow() {
            Some(ref uri) => self.webview_info.set_text(&uri),
            None => self.webview_info.set_text(""),
        }
    }
}

pub fn setup(app: &app::Handle) {
    use gtk::{ SizeGroupExt, BoxExt, LabelExt, WidgetExt };
    use pango;

    let bar = try_extract!(app.status_bar());
    let page_store = try_extract!(app.page_store());

    bar.page_tree_status().pack_start(&bar.page_counter(), true, true, 0);
    bar.page_tree_status().set_margin_top(3);
    bar.page_tree_status().set_margin_bottom(3);

    bar.webview_status().pack_start(&bar.webview_info(), true, true, 0);
    bar.webview_status().set_margin_top(3);
    bar.webview_status().set_margin_bottom(3);

    bar.webview_info().set_halign(gtk::Align::Start);
    bar.webview_info().set_ellipsize(pango::EllipsizeMode::End);

    bar.size_group().add_widget(&bar.page_tree_status());
    bar.size_group().add_widget(&bar.webview_status());

    update_counter(&app, page_store.get_count());
    page_store.on_count_change(with_cloned!(app, move |_page_store, &count| {
        update_counter(&app, count);
    }));
}

fn update_counter(app: &app::Handle, count: usize) {
    use gtk::{ LabelExt };

    let status_bar = try_extract!(app.status_bar());
    status_bar.page_counter().set_text(&format!("{} {}",
        count,
        if count == 1 { "page" } else { "pages" },
    ));
}
