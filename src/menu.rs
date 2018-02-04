
use gio;
use glib;

use app;

pub fn add<F>(parent: &gio::Menu, title: &str, add_items: F) where F: FnOnce(&gio::Menu) {
    use gio::{ MenuExt };

    let menu = gio::Menu::new();
    parent.append_submenu(title, &menu);
    add_items(&menu);
}

pub fn add_item(parent: &gio::Menu, title: &str, action: &str, accel: Option<&str>) {
    use gio::{ MenuExt, MenuItemExt };

    let item = gio::MenuItem::new(title, action);
    if let Some(accel) = accel {
        item.set_attribute_value("accel", Some(&accel.to_string().into()));
    }
    parent.append_item(&item);
}

pub fn add_section<F>(parent: &gio::Menu, add_items: F) where F: FnOnce(&gio::Menu) {
    use gio::{ MenuExt };

    let menu = gio::Menu::new();
    parent.append_section(None, &menu);
    add_items(&menu);
}

pub fn build<F>(add_items: F) -> gio::Menu where F: FnOnce(&gio::Menu) {
    let menu = gio::Menu::new();
    add_items(&menu);
    menu
}

pub fn setup_win_action<F>(
    app: &app::Handle,
    action: &gio::SimpleAction,
    enabled: bool,
    activate: F,
) where F: Fn(&app::Handle, &gio::SimpleAction) + 'static {
    use gio::{ SimpleActionExt, ActionMapExt };

    let window = try_extract!(app.window());

    let app = app.clone();
    action.connect_activate(move |action, _| activate(&app, action));
    action.set_enabled(enabled);

    window.add_action(action);
}

pub fn setup_param_action<F, T>(
    app: &app::Handle,
    action: &gio::SimpleAction,
    enabled: bool,
    activate: F,
) where
    F: Fn(&app::Handle, T) + 'static,
    T: glib::variant::FromVariant,
{
    use gio::{ SimpleActionExt, ActionMapExt };

    let application = try_extract!(app.application());

    let app = app.clone();
    action.connect_activate(move |_, param|
        activate(&app, param.as_ref().and_then(|var| var.get()).unwrap())
    );
    action.set_enabled(enabled);

    application.add_action(action);
}

pub fn setup_action<F>(
    app: &app::Handle,
    action: &gio::SimpleAction,
    enabled: bool,
    activate: F,
) where F: Fn(&app::Handle) + 'static {
    use gio::{ SimpleActionExt, ActionMapExt };

    let application = try_extract!(app.application());

    let app = app.clone();
    action.connect_activate(move |_, _| activate(&app));
    action.set_enabled(enabled);

    application.add_action(action);
}

