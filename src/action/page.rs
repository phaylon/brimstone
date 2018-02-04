
use gtk;

use app;
use page_store;
use action;
use page_tree_view;
use window;
use text;

pub struct Close {
    pub id: page_store::Id,
    pub close_children: Option<bool>,
}

impl app::Perform for Close {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ TreeSelectionExt, TreeViewExt };

        let page_store = try_extract!(app.page_store());
        let page_tree_view = try_extract!(app.page_tree_view());
        let window = try_extract!(app.window());

        let close_children =
            if let Some(close_children) = self.close_children {
                close_children
            } else if let Some(count) = page_store.has_children(self.id) {
                const CLOSE_ALL: i32 = 1;
                const CLOSE_ONE: i32 = 2;
                const CANCEL: i32 = 3;
                let answer = window::confirm_action(
                    &window,
                    &format!("Do you want to close {} pages?", count + 1),
                    &[
                        ("Close Current", CLOSE_ONE),
                        ("Close All", CLOSE_ALL),
                        ("Cancel", CANCEL),
                    ],
                    CLOSE_ONE,
                );
                match answer {
                    CLOSE_ALL => true,
                    CLOSE_ONE => false,
                    _ => return,
                }
            } else {
                false
            };

        let active_id = app.get_active();

        let position =
            if Some(self.id) == active_id {
                page_store.position(self.id)
            } else {
                None
            };

        let select = 
            if let Some((parent, position)) = position {
                let select =
                    if let Some(id) = page_store.find_previous(parent, position) { id }
                    else if let Some(id) = page_store.find_next_incl(parent, position + 1) { id }
                    else {
                        page_store.insert(page_store::InsertData {
                            uri: "about:blank".into(),
                            title: Some("about:blank".into()),
                            parent: None,
                            position: page_store::InsertPosition::End,
                        }).expect("created fallback page")
                    };
                Some(select)
            } else {
                None
            };

        app.without_select(|| page_store.close(self.id, close_children));
        
        if let Some(select) = select {
            page_tree_view.get_selection().unselect_all();
            page_tree_view::select_id(&page_store.tree_store(), &page_tree_view, select);
        }

    }
}

pub struct Select {
    pub id: page_store::Id,
}

impl app::Perform for Select {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ WidgetExt, BoxExt, EntryExt };
        use navigation_bar;
        use app_action;

        if app.is_select_ignored() {
            return;
        }

        let page_store = try_extract!(app.page_store());
        let view_space = try_extract!(app.view_space());
        let nav_bar = try_extract!(app.navigation_bar());

        let new_view = try_extract!(page_store.get_view(self.id, app));
        let title = page_store.get_title(self.id);
        let uri = page_store.get_uri(self.id);

        page_store.set_read(self.id);

        match app.active_webview() {
            Some(webview) => webview.hide(),
            None => (),
        }
        app.set_active(self.id, new_view.clone());
        navigation_bar::adjust_for_load_state(app, page_store.get_load_state(self.id).unwrap());
        app_action::adjust_for_load_state(app, page_store.get_load_state(self.id).unwrap());

        app.perform(action::window::SetTitle {
            title: title.as_ref().map(|title| &title[..]),
            uri: uri.as_ref().map(|uri| &uri[..]).unwrap_or(""),
        });
        nav_bar.address_entry().set_text(&match page_store.get_uri(self.id) {
            Some(uri) => uri,
            None => text::RcString::new(),
        });
        if new_view.get_parent().is_none() {
            view_space.pack_start(&new_view, true, true, 0);
        }
        view_space.show();
        new_view.show_all();
    }
}
