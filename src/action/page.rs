
use app;
use page_store;
use action;

pub struct UpdateCounter;

impl app::Perform for UpdateCounter {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ LabelExt };

        let page_store = try_extract!(app.page_store());
        let status_bar = try_extract!(app.status_bar());

        status_bar.page_counter().set_text(&format!("{}", page_store.get_count()));
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
        let result = page_store.insert(page_store::InsertData {
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
        use gtk::{ ContainerExt, WidgetExt, BoxExt, EntryExt };

        let page_store = try_extract!(app.page_store());
        let view_space = try_extract!(app.view_space());
        let nav_bar = try_extract!(app.navigation_bar());

        let new_view = try_extract!(page_store.get_view(self.id, app));
        let title = page_store.get_title(self.id);
        let uri = page_store.get_uri(self.id);

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
        use gtk::{ WidgetExt };
        use gio::{ SimpleActionExt };

        let nav_bar = try_extract!(app.navigation_bar());
        let app_actions = try_extract!(app.app_actions());

        nav_bar.reload_button().set_visible(!self.state.is_loading);
        nav_bar.stop_button().set_visible(self.state.is_loading);

        app_actions.go_back_action.set_enabled(self.state.can_go_back);
        app_actions.go_forward_action.set_enabled(self.state.can_go_forward);

        app_actions.reload_action.set_enabled(!self.state.is_loading);
        app_actions.reload_bp_action.set_enabled(!self.state.is_loading);
        app_actions.stop_loading_action.set_enabled(self.state.is_loading);
    }
}
