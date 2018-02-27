
use gtk;

use app;
use history;

pub enum Section {
    History,
    Bookmarks,
    Shortcuts,
}

pub struct Map {
    container: gtk::Notebook,
    history: history::Map,
}

impl Map {

    pub fn new() -> Map {
        Map {
            container: create_container(),
            history: history::Map::new(),
        }
    }

    pub fn container(&self) -> &gtk::Notebook { &self.container }
    pub fn history(&self) -> &history::Map { &self.history }

    pub fn show_section(&self, section: Section) {
        use gtk::{ NotebookExt, WidgetExt };

        self.container.set_property_page(match section {
            Section::History => 0,
            Section::Bookmarks => 1,
            Section::Shortcuts => 2,
        });
        self.container.show();
        match section {
            Section::History => self.history.focus(),
            _ => (),
        }
    }

    pub fn hide(&self) {
        use gtk::{ WidgetExt };

        self.container.hide();
    }
}

fn create_container() -> gtk::Notebook {

    let notebook = gtk::Notebook::new();
    notebook
}

pub fn setup(app: &app::Handle) {
    use gtk::{ WidgetExt };

    fn setup_page<W>(map: &Map, title: &str, widget: &W)
    where W: gtk::IsA<gtk::Widget> + gtk::WidgetExt {
        use gtk::prelude::{ NotebookExtManual };

        let label = gtk::Label::new(title);
        label.show_all();
        widget.show_all();

        map.container.append_page(widget, Some(&label));
    }

    let map = try_extract!(app.stored());

    setup_page(&map, "History", map.history.container());
    setup_page(&map, "Bookmarks", &gtk::Box::new(gtk::Orientation::Horizontal, 0));
    setup_page(&map, "Shortcuts", &gtk::Box::new(gtk::Orientation::Horizontal, 0));
    
    map.container.show_all();
    map.container.set_no_show_all(true);
    map.hide();
}
