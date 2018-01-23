
use app;
use page_store;
use page_tree_store;

pub struct Create {
    pub url: String,
    pub title: Option<String>,
    pub parent: Option<page_store::Id>,
}

impl app::Perform for Create {

    type Result = page_store::Id;

    fn perform(self, data: &app::Data) -> page_store::Id {
        let Create { url, title, parent } = self;

        let page_store = &data.page_store;
        let id = page_store.add(url, title.clone());
        let title = page_store.title_for(id).unwrap().to_owned();

        page_tree_store::insert(&data.page_tree_store, None, page_tree_store::Entry {
            id,
            title,
        });

        id
    }
}

pub struct Select {
    pub id: page_store::Id,
}

impl app::Perform for Select {

    type Result = ();

    fn perform(self, data: &app::Data) {
        use gtk::{ ContainerExt, WidgetExt, BoxExt, GtkWindowExt, EntryExt };

        let new_view = match data.page_store.view_for(self.id, data) {
            Some(view) => view,
            None => return,
        };

        data.active_page_store_id.set(Some(self.id));
        *data.active_webview.borrow_mut() = Some(new_view.clone());

        for child in data.view_space.get_children() {
            data.view_space.remove(&child);
        }

        data.window.set_title(&match data.page_store.title_for(self.id) {
            Some(ref title) => format!("{} - Brimstone", title),
            None => "Brimstone".into(),
        });
        data.navigation_bar.address_entry.set_text(&match data.page_store.url_for(self.id) {
            Some(url) => url,
            None => String::new(),
        });
        data.view_space.pack_start(&new_view, true, true, 0);
        data.view_space.show_all();
    }
}
/*
pub struct UpdateTitle {
    pub id: page_store::Id,
    pub title: String,
}

impl app::Perform for UpdateTitle {

    type Result = ();

    fn perform(self, data: &app::Data) {
        use gtk::{ GtkWindowExt };

        data.page_store.set_title_for(self.id, Some(self.title.clone()));

        if let Some(iter) = page_tree_store::find_iter_by_id(&data.page_tree_store, self.id) {
            page_tree_store::set::title(&data.page_tree_store, &iter, self.title.clone());
        }
        if data.is_active(self.id) {
            data.window.set_title(&format!("{} - Brimstone", &self.title));
        }
    }
}
*/
