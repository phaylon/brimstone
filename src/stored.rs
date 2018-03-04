
use gtk;

use app;
use history;
use shortcuts;
use bookmarks;

#[derive(Debug)]
pub enum Section {
    History,
    Bookmarks,
    Shortcuts,
}

pub struct Map {
    container: gtk::Notebook,
    history: history::Map,
    shortcuts: shortcuts::Map,
    bookmarks: bookmarks::Map,
}

impl Map {

    pub fn new() -> Map {
        Map {
            container: create_container(),
            history: history::Map::new(),
            shortcuts: shortcuts::Map::new(),
            bookmarks: bookmarks::Map::new(),
        }
    }

    pub fn container(&self) -> &gtk::Notebook { &self.container }
    pub fn history(&self) -> &history::Map { &self.history }
    pub fn shortcuts(&self) -> &shortcuts::Map { &self.shortcuts }
    pub fn bookmarks(&self) -> &bookmarks::Map { &self.bookmarks }

    pub fn show_section(&self, section: Section) {
        use gtk::prelude::*;

        self.container.set_property_page(match section {
            Section::History => 0,
            Section::Bookmarks => 1,
            Section::Shortcuts => 2,
        });
        self.container.show();
        match section {
            Section::History => self.history.focus(),
            Section::Shortcuts => self.shortcuts.focus(),
            Section::Bookmarks => self.bookmarks.focus(),
        }
    }

    pub fn hide(&self) {
        use gtk::prelude::*;

        self.container.hide();
    }
}

fn create_container() -> gtk::Notebook {

    let notebook = gtk::Notebook::new();
    notebook
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;

    fn setup_page<W>(map: &Map, title: &str, widget: &W)
    where W: gtk::IsA<gtk::Widget> + gtk::WidgetExt {

        let label = gtk::Label::new(title);
        label.show_all();
        widget.show_all();

        map.container.append_page(widget, Some(&label));
    }

    let map = app.stored();

    setup_page(&map, "History", map.history.container());
    setup_page(&map, "Bookmarks", map.bookmarks.container());
    setup_page(&map, "Shortcuts", map.shortcuts.container());
    
    map.container.show_all();
    map.container.set_no_show_all(true);
    map.hide();
}
