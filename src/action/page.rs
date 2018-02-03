
use gtk;

use app;
use page_store;
use action;
use page_tree_view;
use window;

pub struct UpdateCounter;

impl app::Perform for UpdateCounter {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ LabelExt };

        let page_store = try_extract!(app.page_store());
        let status_bar = try_extract!(app.status_bar());

        let count = page_store.get_count();
        status_bar.page_counter().set_text(&format!("{} {}",
            count,
            if count == 1 { "page" } else { "pages" },
        ));
    }
}

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
        let session = try_extract!(app.session_updater());

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
                        app.perform(Create {
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

        app.without_select(|| page_store.close(&session, self.id, close_children));
        
        if let Some(select) = select {
            page_tree_view.get_selection().unselect_all();
            page_tree_view::select_id(&page_store.tree_store(), &page_tree_view, select);
        }

        app.perform(UpdateCounter);
    }
}

pub struct Create {
    pub uri: String,
    pub title: Option<String>,
    pub parent: Option<page_store::Id>,
    pub position: page_store::InsertPosition,
}

impl app::Perform for Create {

    type Result = Option<page_store::Id>;

    fn perform(self, app: &app::Handle) -> Option<page_store::Id> {
        let Create { uri, title, parent, position } = self;

        let page_store = try_extract!(app.page_store());
        let session = try_extract!(app.session_updater());
        let result = page_store.insert(&session, page_store::InsertData {
            uri,
            title,
            parent,
            position,
        });

        app.perform(UpdateCounter);

        result
    }
}

pub struct Select {
    pub id: page_store::Id,
}

impl app::Perform for Select {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ WidgetExt, BoxExt, EntryExt };

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
        app.perform(AdjustUi { state: page_store.get_load_state(self.id).unwrap() });

        app.perform(action::window::SetTitle {
            title: title.as_ref().map(|title| &title[..]),
            uri: uri.as_ref().map(|uri| &uri[..]).unwrap_or(""),
        });
        nav_bar.address_entry().set_text(&match page_store.get_uri(self.id) {
            Some(uri) => uri,
            None => String::new(),
        });
        if new_view.get_parent().is_none() {
            view_space.pack_start(&new_view, true, true, 0);
        }
        view_space.show();
        new_view.show_all();
    }
}

pub struct LoadStateChange {
    pub id: page_store::Id,
    pub state: page_store::LoadState,
}

impl app::Perform for LoadStateChange {

    type Result = ();
    
    fn perform(self, app: &app::Handle) {

        let page_store = try_extract!(app.page_store());
        page_store.set_load_state(self.id, self.state);
        if app.is_active(self.id) {
            app.perform(AdjustUi { state: self.state });
        }
    }
}

pub struct AdjustUi {
    pub state: page_store::LoadState,
}

impl app::Perform for AdjustUi {

    type Result = ();
    
    fn perform(self, app: &app::Handle) {
        use gtk::{ WidgetExt, EntryExt };
        use gio::{ SimpleActionExt };

        let nav_bar = try_extract!(app.navigation_bar());
        let app_actions = try_extract!(app.app_actions());

        nav_bar.reload_button().set_visible(!self.state.is_loading);
        nav_bar.stop_button().set_visible(self.state.is_loading);

        let (tls_icon, tls_tooltip) = match self.state.tls_state {
            page_store::TlsState::Encrypted =>
                ("security-high", "Security: Encrypted"),
            page_store::TlsState::SelfSigned =>
                ("security-medium", "Security: Self-Signed"),
            page_store::TlsState::Insecure =>
                ("security-low", "Security: Insecure"),
        };
        let address_entry = nav_bar.address_entry();
        address_entry.set_icon_from_icon_name(gtk::EntryIconPosition::Primary, tls_icon);
        address_entry.set_icon_tooltip_text(gtk::EntryIconPosition::Primary, tls_tooltip);

        app_actions.go_back_action.set_enabled(self.state.can_go_back);
        app_actions.go_forward_action.set_enabled(self.state.can_go_forward);

        app_actions.reload_action.set_enabled(!self.state.is_loading);
        app_actions.reload_bp_action.set_enabled(!self.state.is_loading);
        app_actions.stop_loading_action.set_enabled(self.state.is_loading);
    }
}
