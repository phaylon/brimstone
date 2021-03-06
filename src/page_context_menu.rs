
use gtk;
use gio;

use app;
use menu;
use page_store;
use text;
use app_action;

const ACTION_CLOSE: &str = "win.page-ctx-close";
const ACTION_RELOAD: &str = "win.page-ctx-reload";
const ACTION_RELOAD_BP: &str = "win.page-ctx-reload-bp";
const ACTION_EXPAND: &str = "win.page-ctx-expand";
const ACTION_EXPAND_ALL: &str = "win.page-ctx-expand-all";
const ACTION_COLLAPSE: &str = "win.page-ctx-collapse";
const ACTION_DUPLICATE: &str = "win.page-ctx-duplicate";
const ACTION_PIN: &str = "win.page-ctx-pin";

pub struct Map {
    menu: gtk::Menu,
    close_action: gio::SimpleAction,
    reload_action: gio::SimpleAction,
    reload_bp_action: gio::SimpleAction,
    expand_action: gio::SimpleAction,
    expand_all_action: gio::SimpleAction,
    collapse_action: gio::SimpleAction,
    duplicate_action: gio::SimpleAction,
    pin_action: gio::SimpleAction,
}

impl Map {

    pub fn menu(&self) -> gtk::Menu { self.menu.clone() }

    pub fn close_action(&self) -> gio::SimpleAction { self.close_action.clone() }

    pub fn reload_action(&self) -> gio::SimpleAction { self.reload_action.clone() }

    pub fn collapse_action(&self) -> gio::SimpleAction { self.collapse_action.clone() }

    pub fn expand_action(&self) -> gio::SimpleAction { self.expand_action.clone() }

    pub fn expand_all_action(&self) -> gio::SimpleAction { self.expand_all_action.clone() }

    pub fn pin_action(&self) -> gio::SimpleAction { self.pin_action.clone() }

    pub fn update_state(&self, state: State) {
        use gio::prelude::*;
        use gtk::prelude::*;

        self.expand_action.set_enabled(state.has_children && !state.is_expanded);
        self.expand_all_action.set_enabled(state.has_children);
        self.collapse_action.set_enabled(state.has_children && state.is_expanded);
        self.pin_action.set_state(&state.is_pinned.to_variant());
    }
}

pub struct State {
    pub has_children: bool,
    pub is_expanded: bool,
    pub is_pinned: bool,
}

pub fn create() -> Map {
    use gtk::prelude::*;

    Map {
        menu: gtk::Menu::new_from_model(&menu::build(|menu| {
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Close", ACTION_CLOSE, None);
                menu::add_item(menu, "Duplicate", ACTION_DUPLICATE, None);
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Reload", ACTION_RELOAD, None);
                menu::add_item(menu, "Reload Bypassing Cache", ACTION_RELOAD_BP, None);
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Expand", ACTION_EXPAND, None);
                menu::add_item(menu, "Expand All", ACTION_EXPAND_ALL, None);
                menu::add_item(menu, "Collapse", ACTION_COLLAPSE, None);
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Pin to Top", ACTION_PIN, None);
            });
        })),
        close_action: gio::SimpleAction::new("page-ctx-close", None),
        reload_action: gio::SimpleAction::new("page-ctx-reload", None),
        reload_bp_action: gio::SimpleAction::new("page-ctx-reload-bp", None),
        expand_action: gio::SimpleAction::new("page-ctx-expand", None),
        expand_all_action: gio::SimpleAction::new("page-ctx-expand-all", None),
        collapse_action: gio::SimpleAction::new("page-ctx-collapse", None),
        duplicate_action: gio::SimpleAction::new("page-ctx-duplicate", None),
        pin_action: gio::SimpleAction::new_stateful("page-ctx-pin", None, &false.to_variant()),
    }
}

pub fn setup(app: &app::Handle) {
    use gtk::{ MenuExt };
    use gio::prelude::*;

    let map = app.page_context_menu();
    map.menu().set_property_attach_widget(Some(app.page_tree_view().widget()));

    menu::setup_win_action(&app, &map.close_action, true, |app, _| {
        log_action!(ACTION_CLOSE);
        app_action::try_close_page(&app, unwrap_or_return!(app.get_page_tree_target()));
    });

    menu::setup_win_action(&app, &map.pin_action, true, |app, action| {
        log_action!(ACTION_PIN);
        let page_store = app.page_store();
        let id = unwrap_or_return!(app.get_page_tree_target());
        let is_active: bool =
            if let Some(state) = action.get_state() {
                state.get().expect("boolean action state")
            } else {
                false
            };
        let is_active = !is_active;
        page_store.set_pinned(id, is_active);
    });

    menu::setup_win_action(&app, &map.duplicate_action, true, |app, _| {
        log_action!(ACTION_DUPLICATE);
        let page_store = app.page_store();
        let id = unwrap_or_return!(app.get_page_tree_target());
        let parent = page_store.get_parent(id);
        let uri = page_store.get_uri(id);
        let title = page_store.get_title(id);
        page_store.insert(
            page_store::InsertData::new(uri.unwrap_or_else(|| text::RcString::new()))
                .with_title(title)
                .with_parent(parent)
                .with_position(page_store::InsertPosition::After(id))
        ).expect("duplicate page creation");
    });

    menu::setup_win_action(&app, &map.reload_action, true, |app, _| {
        use webkit2gtk::{ WebViewExt };

        log_action!(ACTION_RELOAD);

        let id = unwrap_or_return!(app.get_page_tree_target());
        let page_store = app.page_store();
        let webview = unwrap_or_return!(page_store.try_get_view(id));
        webview.reload();
    });

    menu::setup_win_action(&app, &map.reload_bp_action, true, |app, _| {
        use webkit2gtk::{ WebViewExt };

        log_action!(ACTION_RELOAD_BP);

        let id = unwrap_or_return!(app.get_page_tree_target());
        let page_store = app.page_store();
        let webview = unwrap_or_return!(page_store.try_get_view(id));
        webview.reload_bypass_cache();
    });

    menu::setup_win_action(&app, &map.expand_action, true, |app, _| {
        log_action!(ACTION_EXPAND);
        let id = unwrap_or_return!(app.get_page_tree_target());
        let page_tree_view = app.page_tree_view();
        page_tree_view.expand(id, false);
    });

    menu::setup_win_action(&app, &map.expand_all_action, true, |app, _| {
        log_action!(ACTION_EXPAND_ALL);
        let id = unwrap_or_return!(app.get_page_tree_target());
        let page_tree_view = app.page_tree_view();
        page_tree_view.expand(id, true);
    });

    menu::setup_win_action(&app, &map.collapse_action, true, |app, _| {
        log_action!(ACTION_COLLAPSE);
        let id = unwrap_or_return!(app.get_page_tree_target());
        let page_tree_view = app.page_tree_view();
        page_tree_view.collapse(id);
    });
}
