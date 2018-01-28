
use gio;

use app;
use action;
use page_store;
use page_tree_store;
use page_tree_view;

const ACCEL_RELOAD: &str = "<ctrl>r";
const ACCEL_RELOAD_BP: &str = "<ctrl><shift>r";
const ACCEL_QUIT: &str = "<ctrl>q";
const ACCEL_STOP: &str = "Escape";
const ACCEL_GO_BACK: &str = "<alt>Left";
const ACCEL_GO_FORWARD: &str = "<alt>Right";
const ACCEL_NEW: &str = "<ctrl>t";
const ACCEL_CLOSE: &str = "<ctrl>w";
const ACCEL_FOCUS: &str = "<ctrl>l";

pub const ACTION_QUIT: &str = "app.quit";
pub const ACTION_GO_BACK: &str = "app.go-back";
pub const ACTION_GO_FORWARD: &str = "app.go-forward";
pub const ACTION_RELOAD: &str = "app.reload";
pub const ACTION_RELOAD_BP: &str = "app.reload-bp";
pub const ACTION_STOP: &str = "app.stop-loading";
pub const ACTION_NEW: &str = "app.new-page";
pub const ACTION_CLOSE: &str = "app.close-page";
pub const ACTION_FOCUS: &str = "app.focus";

pub struct Map {
    pub menu_bar: gio::Menu,
    pub quit_action: gio::SimpleAction,
    pub go_back_action: gio::SimpleAction,
    pub go_forward_action: gio::SimpleAction,
    pub stop_loading_action: gio::SimpleAction,
    pub reload_action: gio::SimpleAction,
    pub reload_bp_action: gio::SimpleAction,
    pub new_page_action: gio::SimpleAction,
    pub close_page_action: gio::SimpleAction,
    pub focus_action: gio::SimpleAction,
}

pub fn create() -> Map {
    Map {
        menu_bar: create_menu_bar(),
        quit_action: gio::SimpleAction::new("quit", None),
        go_back_action: gio::SimpleAction::new("go-back", None),
        go_forward_action: gio::SimpleAction::new("go-forward", None),
        stop_loading_action: gio::SimpleAction::new("stop-loading", None),
        reload_action: gio::SimpleAction::new("reload", None),
        reload_bp_action: gio::SimpleAction::new("reload-bp", None),
        new_page_action: gio::SimpleAction::new("new-page", None),
        close_page_action: gio::SimpleAction::new("close-page", None),
        focus_action: gio::SimpleAction::new("focus", None),
    }
}

fn create_menu_bar() -> gio::Menu {
    use gio::{ MenuExt, MenuItemExt };
    use menu;

    menu::build(|menu| {
        menu::add(menu, "_File", |menu| {
            menu::add_item(menu, "_Quit", ACTION_QUIT, Some(ACCEL_QUIT));
        });
        menu::add(menu, "_Page", |menu| {
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "New Page", ACTION_NEW, Some(ACCEL_NEW));
                menu::add_item(menu, "Close Page", ACTION_CLOSE, Some(ACCEL_CLOSE));
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Go to _Previous Page", ACTION_GO_BACK, Some(ACCEL_GO_BACK));
                menu::add_item(menu, "Go to _Next Page", ACTION_GO_FORWARD, Some(ACCEL_GO_FORWARD));
                menu::add_item(menu, "_Reload", ACTION_RELOAD, Some(ACCEL_RELOAD));
                menu::add_item(menu, "Reload Bypassing Cache", ACTION_RELOAD_BP, Some(ACCEL_RELOAD_BP));
                menu::add_item(menu, "_Stop Loading", ACTION_STOP, Some(ACCEL_STOP));
            });
        });
    })
}

pub fn setup(app: app::Handle) {
    use gtk::{ GtkApplicationExt, WidgetExt, GtkWindowExt };
    use gio::{ SimpleActionExt, ActionMapExt };
    use webkit2gtk::{ WebViewExt };
    use menu;

    let application = try_extract!(app.application());
    let app_actions = try_extract!(app.app_actions());

    application.set_menubar(&app_actions.menu_bar);

    menu::setup_action(&app, &app_actions.quit_action, true, |app| {
        let window = try_extract!(app.window());
        window.close();
    });
    menu::setup_action(&app, &app_actions.go_back_action, false, |app| {
        let webview = try_extract!(app.active_webview());
        if webview.can_go_back() {
            webview.go_back();
        }
    });
    menu::setup_action(&app, &app_actions.go_forward_action, false, |app| {
        let webview = try_extract!(app.active_webview());
        if webview.can_go_forward() {
            webview.go_forward();
        }
    });
    menu::setup_action(&app, &app_actions.reload_action, false, |app| {
        let webview = try_extract!(app.active_webview());
        webview.reload();
    });
    menu::setup_action(&app, &app_actions.reload_bp_action, false, |app| {
        let webview = try_extract!(app.active_webview());
        webview.reload_bypass_cache();
    });
    menu::setup_action(&app, &app_actions.stop_loading_action, false, |app| {
        let webview = try_extract!(app.active_webview());
        webview.stop_loading();
    });
    menu::setup_action(&app, &app_actions.new_page_action, true, |app| {
        use gtk::{ TreeViewExt, TreeSelectionExt };

        let page_store = try_extract!(app.page_store());
        let page_tree_store = try_extract!(app.page_tree_store());
        let page_tree_view = try_extract!(app.page_tree_view());
        let id = try_extract!(app.get_active());
        let parent_id = page_store.get_parent(id);
        let new_id = try_extract!(app.perform(action::page::Create {
            title: None,
            uri: "about:blank".into(),
            parent: parent_id,
            position: page_store::InsertPosition::After(id),
        }));

        page_tree_view::select_id(&page_tree_store, &page_tree_view, new_id);

        let navigation_bar = try_extract!(app.navigation_bar());
        navigation_bar.address_entry().grab_focus();
    });
    menu::setup_action(&app, &app_actions.focus_action, true, |app| {
        let navigation_bar = try_extract!(app.navigation_bar());
        navigation_bar.address_entry().grab_focus();
    });
    application.add_accelerator(ACCEL_FOCUS, ACTION_FOCUS, None);
    menu::setup_action(&app, &app_actions.close_page_action, true, |app| {
        app.perform(action::page::Close {
            id: try_extract!(app.get_active()),
            close_children: None,
        });
    });
}


