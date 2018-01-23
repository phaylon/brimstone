
use gio;

use app;

pub struct Map {
    pub menu_bar: gio::Menu,
    pub quit_action: gio::SimpleAction,
}

pub fn create() -> Map {
    Map {
        menu_bar: create_menu_bar(),
        quit_action: gio::SimpleAction::new("quit", None),
    }
}

fn create_menu_bar() -> gio::Menu {
    use gio::{ MenuExt, MenuItemExt };

    let menu = gio::Menu::new();
    let menu_file = gio::Menu::new();

    menu.append_submenu("_File", &menu_file);

    let quit_action = gio::MenuItem::new("_Quit", "app.quit");
    quit_action.set_attribute_value("accel", Some(&"<ctrl>q".to_string().into()));
    menu_file.append_item(&quit_action);

    menu
}

pub fn setup(app: app::Handle) {
    use gtk::{ GtkApplicationExt, WidgetExt, GtkWindowExt };
    use gio::{ SimpleActionExt, ActionMapExt };

    let application = app.application().unwrap();
    let app_actions = app.app_actions().unwrap();
    let window = app.window().unwrap();

    application.set_menubar(&app_actions.menu_bar);

    app_actions.quit_action.connect_activate(with_cloned!(window, move |_, _| {
        window.close();
    }));
    application.add_action(&app_actions.quit_action);
    application.add_accelerator("<ctrl>q", "app.quit", None);
}


