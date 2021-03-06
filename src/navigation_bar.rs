
use gtk;
use gdk;

use app;
use app_action;
use bar;
use mouse;
use page_store;
use text;

pub struct Map {
    pub container: gtk::Box,
    pub address_entry: gtk::Entry,
    pub go_back_button: gtk::Button,
    pub go_forward_button: gtk::Button,
    pub reload_button: gtk::Button,
    pub stop_button: gtk::Button,
    pub domain_button: gtk::Button,
    pub bookmarks_button: gtk::Button,
}

impl Map {

    pub fn new() -> Map {
        Map {
            container: bar::create_container(),
            address_entry:bar::create_address_entry(),
            go_back_button: bar::create_nav_button("go-previous", false, true),
            go_forward_button: bar::create_nav_button("go-next", false, true),
            reload_button: bar::create_nav_button("view-refresh", true, true),
            stop_button: bar::create_nav_button("process-stop", true, false),
            domain_button: bar::create_nav_button("network-server", true, true),
            bookmarks_button: bar::create_nav_button("user-bookmarks", true, true),
        }
    }

    pub fn container(&self) -> gtk::Box { self.container.clone() }

    pub fn address_entry(&self) -> gtk::Entry { self.address_entry.clone() }

    pub fn go_back_button(&self) -> gtk::Button { self.go_back_button.clone() }

    pub fn go_forward_button(&self) -> gtk::Button { self.go_forward_button.clone() }

    pub fn stop_button(&self) -> gtk::Button { self.stop_button.clone() }

    pub fn reload_button(&self) -> gtk::Button { self.reload_button.clone() }

    pub fn bookmarks_button(&self) -> gtk::Button { self.bookmarks_button.clone() }
}

#[derive(Copy, Clone)]
enum NavPopupMode { Back, Forward }

fn show_nav_popup(
    app: &app::Handle,
    event: &gdk::EventButton,
    mode: NavPopupMode,
) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt, BackForwardListExt, BackForwardListItemExt };

    const LIMIT: u32 = 10;

    let webview = unwrap_or_return!(app.active_webview());
    let bf_list = unwrap_or_return!(webview.get_back_forward_list());

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
            let webview = unwrap_or_return!(app.active_webview());
            webview.go_to_back_forward_list_item(&item);
        }));
        menu.append(&menu_item);
    }

    app.set_cached_nav_menu(Some(menu.clone()));
    menu.show_all();
    menu.popup_easy(event.get_button(), event.get_time());
}

fn show_domain_popup(app: &app::Handle, event: &gdk::EventButton) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let webview = unwrap_or_return!(app.active_webview());
    let page_id = webview.get_page_id();
    let page_state_store = app.page_state_store();
    let data = unwrap_or_return!(
        page_state_store.lock()
            .expect("exclusive access to page state store")
            .get_data(page_id)
    );
    let domains = app.domain_settings();

    let menu = gtk::Menu::new();
    let source_hosts = data.host().to_expanded();
    let mut complete = false;
    for host in &source_hosts {
        if domains.has_always_entry(host) {
            let item = gtk::MenuItem::new_with_label(
                &format!("Unallow All Requests from {}", host.as_str()),
            );
            menu.append(&item);
            complete = true;
            let host = host.clone();
            item.connect_activate(with_cloned!(app, move |_item| {
                let domains = app.domain_settings();
                domains.remove_always_entry(&host);
            }));
        }
    }

    if !complete {

        for host in &source_hosts {
            let item = gtk::MenuItem::new_with_label(
                &format!("Allow All Requests from {}", host.as_str()),
            );
            menu.append(&item);
            let host = host.clone();
            item.connect_activate(with_cloned!(app, move |_item| {
                let domains = app.domain_settings();
                domains.insert_always_entry(&host);
            }));
        }

        let mut target_hosts = Vec::new();
        for host in data.allowed() {
            target_hosts.extend(host.to_expanded());
        }
        for host in data.denied() {
            target_hosts.extend(host.to_expanded());
        }
        target_hosts.sort();
        target_hosts.dedup();

        let target_hosts = target_hosts.into_iter().map(|host| {
            let has_entry = domains.has_entry(data.host(), &host);
            (host, has_entry)
        }).collect::<Vec<_>>();

        if !target_hosts.is_empty() {
            let sep = gtk::SeparatorMenuItem::new();
            menu.append(&sep);
        }

        let mut allowed_count = 0;
        let mut denied_count = 0;
        for &(ref host, has_entry) in &target_hosts {
            if !has_entry {
                let item = gtk::MenuItem::new_with_label(
                    &format!("Allow Requests to {}", host.as_str()),
                );
                menu.append(&item);
                let host = host.clone();
                let source = data.host().clone();
                item.connect_activate(with_cloned!(app, move |_item| {
                    let domains = app.domain_settings();
                    domains.insert_entry(&source, &host);
                }));
                denied_count += 1;
            } else {
                allowed_count += 1;
            }
        }

        if allowed_count > 0 && denied_count > 0 {
            let sep = gtk::SeparatorMenuItem::new();
            menu.append(&sep);
        }

        for &(ref host, has_entry) in &target_hosts {
            if has_entry {
                let item = gtk::MenuItem::new_with_label(
                    &format!("Unallow Requests to {}", host.as_str()),
                );
                menu.append(&item);
                let host = host.clone();
                let source = data.host().clone();
                item.connect_activate(with_cloned!(app, move |_item| {
                    let domains = app.domain_settings();
                    domains.remove_entry(&source, &host);
                }));
            }
        }
    }

    app.set_cached_domain_menu(Some(menu.clone()));
    menu.show_all();
    menu.popup_easy(event.get_button(), event.get_time());
}

fn is_plain_host(value: &str) -> bool {
    value.chars().all(|c| match c {
        'a'...'z' | 'A'...'Z' | '0'...'9' | '.' | '-' => true,
        _ => false,
    })
}

fn is_shortcut(value: &str) -> Option<(&str, &str)> {
    let value = value.trim();
    Some(match value.find(' ') {
        Some(index) => (value[..index].trim(), value[index..].trim()),
        None => return None,
    })
}

fn resolve_uri(app: &app::Handle, uri: &str) -> String {

    if let Some((name, value)) = is_shortcut(uri) {
        let shortcuts = app.shortcuts();
        if let Some(pattern) = shortcuts.get(&name) {
            return pattern.replace("%s", value);
        }
    }

    if is_plain_host(&uri) {
        return format!("http://{}", uri);
    }

    uri.into()
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;

    let bar = app.navigation_bar();
    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();

    bar.container.pack_start(&bar.go_back_button, false, true, 0);
    bar.container.pack_start(&bar.go_forward_button, false, true, 0);
    bar.container.pack_start(&bar.address_entry, true, true, 0);
    bar.container.pack_start(&bar.reload_button, false, true, 0);
    bar.container.pack_start(&bar.stop_button, false, true, 0);
    bar.container.pack_start(&bar.bookmarks_button, false, true, 0);
    bar.container.pack_start(&bar.domain_button, false, true, 0);

    bar.domain_button.connect_button_release_event(with_cloned!(app, move |_button, event| {
        show_domain_popup(&app, event);
        gtk::prelude::Inhibit(false)
    }));

    bar.address_entry.connect_activate(with_cloned!(app, move |entry| {
        log_debug!("address entry activated");
        address_activate(&app, entry);
    }));

    bar.go_back_button.set_action_name(Some(app_action::ACTION_GO_BACK));
    bar.go_forward_button.set_action_name(Some(app_action::ACTION_GO_FORWARD));

    let attach_nav_popup = |button: &gtk::Button, mode| {
        button.connect_button_release_event(with_cloned!(app, move |_button, event| {
            if event.get_button() == mouse::BUTTON_RIGHT {
                log_debug!("show navigation context menu");
                show_nav_popup(&app, event, mode);
            }
            gtk::prelude::Inhibit(false)
        }));
    };
    attach_nav_popup(&bar.go_back_button, NavPopupMode::Back);
    attach_nav_popup(&bar.go_forward_button, NavPopupMode::Forward);

    bar.stop_button.set_action_name(Some(app_action::ACTION_STOP));

    bar.reload_button.connect_button_release_event(with_cloned!(app, move |_button, event| {
        reload(&app, event);
        gtk::prelude::Inhibit(false)
    }));

    app.bar_size_group().add_widget(&bar.container);

    page_store.on_load_state_change(with_cloned!(app, move |_page_store, &(id, state)| {
        on_load_state_change(&app, id, state);
    }));

    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {
        on_selection_change(&app, id);
    }));
}

fn address_activate(app: &app::Handle, entry: &gtk::Entry) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };

    let webview = unwrap_or_return!(app.active_webview());
    let uri = unwrap_or_return!(entry.get_text());
    let uri = resolve_uri(&app, &uri);

    webview.load_uri(&uri);
}

fn reload(app: &app::Handle, event: &gdk::EventButton) {
    use gio::prelude::*;

    let app_actions = app.app_actions();
    if event.get_state() == gdk::ModifierType::SHIFT_MASK {
        app_actions.reload_bp_action.activate(None);
    } else {
        app_actions.reload_action.activate(None);
    }
}

fn on_load_state_change(app: &app::Handle, id: page_store::Id, state: page_store::LoadState) {
    if app.is_active(id) {
        adjust_for_load_state(&app, state);
    }
}

fn on_selection_change(app: &app::Handle, id: page_store::Id) {
    use gtk::prelude::*;

    let page_store = app.page_store();
    let load_state = unwrap_or_return!(page_store.get_load_state(id));
    let nav_bar = app.navigation_bar();

    adjust_for_load_state(&app, load_state);
    
    nav_bar.address_entry().set_text(&match page_store.get_uri(id) {
        Some(uri) => uri,
        None => text::RcString::new(),
    });
}

pub fn adjust_for_load_state(app: &app::Handle, state: page_store::LoadState) {
    use gtk::prelude::*;

    let nav_bar = app.navigation_bar();

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
