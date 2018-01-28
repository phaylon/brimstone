
use gtk;
use gio;

use app;
use menu;
use action;

const ACTION_CLOSE: &str = "win.page-ctx-close";
const ACTION_CLOSE_ALL: &str = "win.page-ctx-close-all";
const ACTION_RELOAD: &str = "win.page-ctx-reload";
const ACTION_RELOAD_BP: &str = "win.page-ctx-reload-bp";

pub struct Map {
    menu: gtk::Menu,
    close_action: gio::SimpleAction,
    close_all_action: gio::SimpleAction,
    reload_action: gio::SimpleAction,
    reload_bp_action: gio::SimpleAction,
}

impl Map {

    pub fn menu(&self) -> gtk::Menu { self.menu.clone() }

    pub fn close_action(&self) -> gio::SimpleAction { self.close_action.clone() }

    pub fn close_all_action(&self) -> gio::SimpleAction { self.close_all_action.clone() }

    pub fn reload_action(&self) -> gio::SimpleAction { self.reload_action.clone() }
}

pub fn create() -> Map {
    Map {
        menu: gtk::Menu::new_from_model(&menu::build(|menu| {
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Close", ACTION_CLOSE, None);
                menu::add_item(menu, "Close Tree", ACTION_CLOSE_ALL, None);
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Reload", ACTION_RELOAD, None);
                menu::add_item(menu, "Reload Bypassing Cache", ACTION_RELOAD_BP, None);
            });
        })),
        close_action: gio::SimpleAction::new("page-ctx-close", None),
        close_all_action: gio::SimpleAction::new("page-ctx-close-all", None),
        reload_action: gio::SimpleAction::new("page-ctx-reload", None),
        reload_bp_action: gio::SimpleAction::new("page-ctx-reload-bp", None),
    }
}

pub fn setup(app: app::Handle) {
    use gtk::{ MenuExt };

    let map = try_extract!(app.page_context_menu());
    map.menu().set_property_attach_widget(Some(&try_extract!(app.page_tree_view())));

    menu::setup_win_action(&app, &map.close_action, true, |app| {
        app.perform(action::page::Close {
            id: try_extract!(app.get_page_tree_target()),
            close_children: None,
        });
    });

    menu::setup_win_action(&app, &map.close_all_action, true, |app| {
        app.perform(action::page::Close {
            id: try_extract!(app.get_page_tree_target()),
            close_children: Some(true),
        });
    });

    menu::setup_win_action(&app, &map.reload_action, true, |app| {
        use webkit2gtk::{ WebViewExt };

        let id = try_extract!(app.get_page_tree_target());
        let page_store = try_extract!(app.page_store());
        let webview = try_extract!(page_store.try_get_view(id));
        webview.reload();
    });

    menu::setup_win_action(&app, &map.reload_bp_action, true, |app| {
        use webkit2gtk::{ WebViewExt };

        let id = try_extract!(app.get_page_tree_target());
        let page_store = try_extract!(app.page_store());
        let webview = try_extract!(page_store.try_get_view(id));
        webview.reload_bypass_cache();
    });
}
