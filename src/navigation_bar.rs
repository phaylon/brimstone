
use std::rc;

use gtk;
use gdk;

use app;
use app_action;
use bar;
use mouse;
use page_store;
use text;

pub struct Bar {
    pub container: gtk::Box,
    pub address_entry: gtk::Entry,
    pub go_back_button: gtk::Button,
    pub go_forward_button: gtk::Button,
    pub reload_button: gtk::Button,
    pub stop_button: gtk::Button,
}

pub struct Handle {
    bar: rc::Rc<Bar>,
}

impl Handle {

    pub fn new(bar: rc::Rc<Bar>) -> Handle {
        Handle { bar }
    }

    pub fn container(&self) -> gtk::Box { self.bar.container.clone() }

    pub fn address_entry(&self) -> gtk::Entry { self.bar.address_entry.clone() }

    pub fn go_back_button(&self) -> gtk::Button { self.bar.go_back_button.clone() }

    pub fn go_forward_button(&self) -> gtk::Button { self.bar.go_forward_button.clone() }

    pub fn stop_button(&self) -> gtk::Button { self.bar.stop_button.clone() }

    pub fn reload_button(&self) -> gtk::Button { self.bar.reload_button.clone() }
}

pub fn create() -> Bar {
    Bar {
        container: bar::create_container(),
        address_entry:bar::create_address_entry(),
        go_back_button: bar::create_nav_button("go-previous", false, true),
        go_forward_button: bar::create_nav_button("go-next", false, true),
        reload_button: bar::create_nav_button("view-refresh", true, true),
        stop_button: bar::create_nav_button("process-stop", true, false),
    }
}

#[derive(Copy, Clone)]
enum NavPopupMode { Back, Forward }

fn show_nav_popup(
    app: &app::Handle,
    event: &gdk::EventButton,
    mode: NavPopupMode,
) {
    use webkit2gtk::{ WebViewExt, BackForwardListExt, BackForwardListItemExt };
    use gtk::{ MenuItemExt, MenuShellExt, WidgetExt };
    use gtk::prelude::{ MenuExtManual };

    const LIMIT: u32 = 10;

    let webview = try_extract!(app.active_webview());
    let bf_list = try_extract!(webview.get_back_forward_list());

    let items = match mode {
        NavPopupMode::Back => bf_list.get_back_list_with_limit(LIMIT),
        NavPopupMode::Forward => {
            let mut items = bf_list.get_forward_list_with_limit(LIMIT);
            items.reverse();
            items
        },
    };

    let menu = gtk::Menu::new();
    for item in items {
        let title = item
            .get_title()
            .unwrap_or_else(|| item.get_uri().unwrap_or_else(|| String::new()));
        let menu_item = gtk::MenuItem::new_with_label(&title);
        menu_item.connect_activate(with_cloned!(app, move |_menu_item| {
            let webview = try_extract!(app.active_webview());
            webview.go_to_back_forward_list_item(&item);
        }));
        menu.append(&menu_item);
    }

    app.set_cached_nav_menu(Some(menu.clone()));
    menu.show_all();
    menu.popup_easy(event.get_button(), event.get_time());
}

pub fn setup(app: app::Handle) {
    use gtk::{ BoxExt, EntryExt, WidgetExt, ActionableExt, SizeGroupExt };
    use webkit2gtk::{ WebViewExt };
    use gio::{ ActionExt };

    let bar = app.navigation_bar().unwrap().bar;
    let page_store = app.page_store().unwrap();
    let page_tree_view = app.page_tree_view().unwrap();

    bar.container.pack_start(&bar.go_back_button, false, true, 0);
    bar.container.pack_start(&bar.go_forward_button, false, true, 0);
    bar.container.pack_start(&bar.address_entry, true, true, 0);
    bar.container.pack_start(&bar.reload_button, false, true, 0);
    bar.container.pack_start(&bar.stop_button, false, true, 0);

    bar.address_entry.connect_activate(with_cloned!(app, move |entry| {
        let uri = try_extract!(entry.get_text());
        let webview = try_extract!(app.active_webview());
        webview.load_uri(&uri);
    }));

    bar.go_back_button.set_action_name(Some(app_action::ACTION_GO_BACK));
    bar.go_forward_button.set_action_name(Some(app_action::ACTION_GO_FORWARD));

    let attach_nav_popup = |button: &gtk::Button, mode| {
        button.connect_button_release_event(with_cloned!(app, move |_button, event| {
            if event.get_button() == mouse::BUTTON_RIGHT {
                show_nav_popup(&app, event, mode);
            }
            gtk::prelude::Inhibit(false)
        }));
    };
    attach_nav_popup(&bar.go_back_button, NavPopupMode::Back);
    attach_nav_popup(&bar.go_forward_button, NavPopupMode::Forward);

    bar.stop_button.set_action_name(Some(app_action::ACTION_STOP));

    bar.reload_button.connect_button_release_event(with_cloned!(app, move |_button, event| {
        fn_scope! {
            let app_actions = try_extract!(app.app_actions());
            if event.get_state() == gdk::ModifierType::SHIFT_MASK {
                app_actions.reload_bp_action.activate(None);
            } else {
                app_actions.reload_action.activate(None);
            }
        };
        gtk::prelude::Inhibit(false)
    }));

    app.bar_size_group().unwrap().add_widget(&bar.container);

    page_store.on_load_state_change(with_cloned!(app, move |_page_store, &(id, state)| {
        if app.is_active(id) {
            adjust_for_load_state(&app, state);
        }
    }));

    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {

        let page_store = try_extract!(app.page_store());
        let load_state = try_extract!(page_store.get_load_state(id));
        let nav_bar = try_extract!(app.navigation_bar());

        adjust_for_load_state(&app, load_state);
        
        nav_bar.address_entry().set_text(&match page_store.get_uri(id) {
            Some(uri) => uri,
            None => text::RcString::new(),
        });
    }));
}

pub fn adjust_for_load_state(app: &app::Handle, state: page_store::LoadState) {
    use gtk::{ WidgetExt, EntryExt };

    let nav_bar = try_extract!(app.navigation_bar());

    nav_bar.reload_button().set_visible(!state.is_loading);
    nav_bar.stop_button().set_visible(state.is_loading);

    let (tls_icon, tls_tooltip) = match state.tls_state {
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
}
