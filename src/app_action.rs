
use gio;

use app;
use page_store;
use window;
use recently_closed;
use stored;

const ACCEL_RELOAD: &str = "<ctrl>r";
const ACCEL_RELOAD_BP: &str = "<ctrl><shift>r";
const ACCEL_QUIT: &str = "<ctrl>q";
const ACCEL_STOP: &str = "Escape";
const ACCEL_GO_BACK: &str = "<alt>Left";
const ACCEL_GO_FORWARD: &str = "<alt>Right";
const ACCEL_NEW: &str = "<ctrl>t";
const ACCEL_CLOSE: &str = "<ctrl>w";
const ACCEL_FOCUS: &str = "<ctrl>l";
const ACCEL_RECENT_REOPEN: &str = "<ctrl><shift>t";

pub const ACTION_QUIT: &str = "app.quit";
pub const ACTION_GO_BACK: &str = "app.go-back";
pub const ACTION_GO_FORWARD: &str = "app.go-forward";
pub const ACTION_RELOAD: &str = "app.reload";
pub const ACTION_RELOAD_BP: &str = "app.reload-bp";
pub const ACTION_STOP: &str = "app.stop-loading";
pub const ACTION_NEW: &str = "app.new-page";
pub const ACTION_NEW_CHILD: &str = "app.new-child-page";
pub const ACTION_CLOSE: &str = "app.close-page";
pub const ACTION_FOCUS: &str = "app.focus";
pub const ACTION_RECENT_REOPEN: &str = "app.recent-reopen";
pub const ACTION_REOPEN: &str = "app.reopen";
pub const ACTION_HISTORY: &str = "app.show-history";
pub const ACTION_BOOKMARKS: &str = "app.show-bookmarks";
pub const ACTION_SHORTCUTS: &str = "app.show-shortcuts";

pub struct Map {
    pub menu_bar: gio::Menu,
    pub quit_action: gio::SimpleAction,
    pub go_back_action: gio::SimpleAction,
    pub go_forward_action: gio::SimpleAction,
    pub stop_loading_action: gio::SimpleAction,
    pub reload_action: gio::SimpleAction,
    pub reload_bp_action: gio::SimpleAction,
    pub new_page_action: gio::SimpleAction,
    pub new_child_page_action: gio::SimpleAction,
    pub close_page_action: gio::SimpleAction,
    pub focus_action: gio::SimpleAction,
    pub recent_reopen_action: gio::SimpleAction,
    pub recent_menu: gio::Menu,
    pub reopen_action: gio::SimpleAction,
    pub history_action: gio::SimpleAction,
    pub bookmarks_action: gio::SimpleAction,
    pub shortcuts_action: gio::SimpleAction,
}

pub fn create() -> Map {
    use gtk::prelude::*;

    let recent_menu = gio::Menu::new();
    let menu_bar = create_menu_bar(&recent_menu);
    Map {
        menu_bar,
        recent_menu,
        quit_action: gio::SimpleAction::new("quit", None),
        go_back_action: gio::SimpleAction::new("go-back", None),
        go_forward_action: gio::SimpleAction::new("go-forward", None),
        stop_loading_action: gio::SimpleAction::new("stop-loading", None),
        reload_action: gio::SimpleAction::new("reload", None),
        reload_bp_action: gio::SimpleAction::new("reload-bp", None),
        new_page_action: gio::SimpleAction::new("new-page", None),
        new_child_page_action: gio::SimpleAction::new("new-child-page", None),
        close_page_action: gio::SimpleAction::new("close-page", None),
        focus_action: gio::SimpleAction::new("focus", None),
        recent_reopen_action: gio::SimpleAction::new("recent-reopen", None),
        reopen_action: gio::SimpleAction::new(
            "reopen",
            Some(&*page_store::Id::static_variant_type()),
        ),
        history_action: gio::SimpleAction::new_stateful(
            "show-history",
            None,
            &false.to_variant(),
        ),
        bookmarks_action: gio::SimpleAction::new_stateful(
            "show-bookmarks",
            None,
            &false.to_variant(),
        ),
        shortcuts_action: gio::SimpleAction::new_stateful(
            "show-shortcuts",
            None,
            &false.to_variant(),
        ),
    }
}

fn create_menu_bar(recent_menu: &gio::Menu) -> gio::Menu {
    use gio::prelude::*;
    use menu;

    menu::build(|menu| {
        menu::add(menu, "_File", |menu| {
            menu::add_item(menu, "_Quit", ACTION_QUIT, Some(ACCEL_QUIT));
        });
        menu::add(menu, "_Page", |menu| {
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "_New Page", ACTION_NEW, Some(ACCEL_NEW));
                menu::add_item(menu, "New Child Page", ACTION_NEW_CHILD, None);
                menu::add_item(menu, "_Close Page", ACTION_CLOSE, Some(ACCEL_CLOSE));
            });
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "Go to _Previous Page", ACTION_GO_BACK, Some(ACCEL_GO_BACK));
                menu::add_item(menu, "Go to _Next Page", ACTION_GO_FORWARD, Some(ACCEL_GO_FORWARD));
                menu::add_item(menu, "_Reload", ACTION_RELOAD, Some(ACCEL_RELOAD));
                menu::add_item(menu, "Reload Bypassing Cache", ACTION_RELOAD_BP, Some(ACCEL_RELOAD_BP));
                menu::add_item(menu, "_Stop Loading", ACTION_STOP, Some(ACCEL_STOP));
            });
        });
        menu::add(menu, "_View", |menu| {
            menu::add_section(menu, |menu| {
                menu::add_item(menu, "_History", ACTION_HISTORY, None);
                menu::add_item(menu, "_Bookmarks", ACTION_BOOKMARKS, None);
                menu::add_item(menu, "_Shortcuts", ACTION_SHORTCUTS, None);
            });
        });
        menu::add(menu, "_History", |menu| {
            menu::add_section(menu, |menu| {
                menu::add(menu, "_Recently Closed Pages", |menu| {
                    menu::add_item(menu,
                        "Re_open Most Recent",
                        ACTION_RECENT_REOPEN,
                        Some(ACCEL_RECENT_REOPEN),
                    );
                    menu.append_section(None, recent_menu);
                });
            });
        });
    })
}

fn change_stored_view(
    app: &app::Handle,
    action: &gio::SimpleAction,
    section: stored::Section,
) {
    use gio::prelude::*;
    use gtk::prelude::*;

    let app_actions = app.app_actions();
    let stored = app.stored();

    let is_active: bool =
        if let Some(state) = action.get_state() {
            state.get().expect("boolean action state")
        } else {
            false
        };

    if is_active {
        stored.hide();
        action.set_state(&false.to_variant());
    } else {
        stored.show_section(section);
        app_actions.history_action.set_state(&false.to_variant());
        app_actions.bookmarks_action.set_state(&false.to_variant());
        app_actions.shortcuts_action.set_state(&false.to_variant());
        action.set_state(&true.to_variant());
    }
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;
    use webkit2gtk::{ WebViewExt };
    use menu;

    let application = app.application();
    let app_actions = app.app_actions();
    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();

    application.set_menubar(&app_actions.menu_bar);

    menu::setup_action(&app, &app_actions.history_action, true, |app, action| {
        log_action!(ACTION_HISTORY);
        change_stored_view(&app, action, stored::Section::History);
    });
    menu::setup_action(&app, &app_actions.bookmarks_action, true, |app, action| {
        log_action!(ACTION_BOOKMARKS);
        change_stored_view(&app, action, stored::Section::Bookmarks);
    });
    menu::setup_action(&app, &app_actions.shortcuts_action, true, |app, action| {
        log_action!(ACTION_SHORTCUTS);
        change_stored_view(&app, action, stored::Section::Shortcuts);
    });

    menu::setup_action(&app, &app_actions.quit_action, true, |app, _| {
        log_action!(ACTION_QUIT);
        app.window().close();
    });
    menu::setup_action(&app, &app_actions.go_back_action, false, |app, _| {
        log_action!(ACTION_GO_BACK);
        let webview = unwrap_or_return!(app.active_webview());
        if webview.can_go_back() {
            webview.go_back();
        }
    });
    menu::setup_action(&app, &app_actions.go_forward_action, false, |app, _| {
        log_action!(ACTION_GO_FORWARD);
        let webview = unwrap_or_return!(app.active_webview());
        if webview.can_go_forward() {
            webview.go_forward();
        }
    });
    menu::setup_action(&app, &app_actions.reload_action, false, |app, _| {
        log_action!(ACTION_RELOAD);
        let webview = unwrap_or_return!(app.active_webview());
        webview.reload();
    });
    menu::setup_action(&app, &app_actions.reload_bp_action, false, |app, _| {
        log_action!(ACTION_RELOAD_BP);
        let webview = unwrap_or_return!(app.active_webview());
        webview.reload_bypass_cache();
    });
    menu::setup_action(&app, &app_actions.stop_loading_action, false, |app, _| {
        log_action!(ACTION_STOP);
        let webview = unwrap_or_return!(app.active_webview());
        webview.stop_loading();
    });
    menu::setup_action(&app, &app_actions.new_child_page_action, true, |app, _| {
        log_action!(ACTION_NEW_CHILD);
        create_new_page(&app, CreateMode::Child);
    });
    menu::setup_action(&app, &app_actions.new_page_action, true, |app, _| {
        log_action!(ACTION_NEW);
        create_new_page(&app, CreateMode::Sibling);
    });
    menu::setup_action(&app, &app_actions.focus_action, true, |app, _| {
        log_action!(ACTION_FOCUS);
        let navigation_bar = app.navigation_bar();
        navigation_bar.address_entry().grab_focus();
    });
    application.add_accelerator(ACCEL_FOCUS, ACTION_FOCUS, None);
    menu::setup_action(&app, &app_actions.close_page_action, true, |app, _| {
        log_action!(ACTION_CLOSE);
        try_close_page(&app, unwrap_or_return!(app.get_active()));
    });
    menu::setup_param_action(&app, &app_actions.reopen_action, true, |app, id: page_store::Id| {
        log_action!(ACTION_REOPEN);
        reopen(&app, Some(id));
    });
    menu::setup_action(&app, &app_actions.recent_reopen_action, false, |app, _| {
        log_action!(ACTION_RECENT_REOPEN);
        reopen(&app, None);
    });

    page_store.recently_closed_state().on_change(with_cloned!(app, move |state, _| {
        use gio::prelude::*;

        let app_actions = app.app_actions();

        let menu = &app_actions.recent_menu;
        menu.remove_all();
        state.iterate_pages(|page| {
            let item = match page.title {
                Some(ref title) => gio::MenuItem::new(title.as_str(), None),
                None => gio::MenuItem::new(page.uri.as_str(), None),
            };
            item.set_action_and_target_value(ACTION_REOPEN, Some(&page.id.to_variant()));
            menu.prepend_item(&item);
        });
        
        app_actions.recent_reopen_action.set_enabled(!state.is_empty());
    }));
    
    page_store.on_load_state_change(with_cloned!(app, move |_page_store, &(id, state)| {
        if app.is_active(id) {
            adjust_for_load_state(&app, state);
        }
    }));

    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {
        let page_store = app.page_store();
        let load_state = unwrap_or_return!(page_store.get_load_state(id));
        adjust_for_load_state(&app, load_state);
    }));
}

#[derive(Debug, Clone, Copy)]
pub enum CreateMode {
    Sibling,
    Child,
}

pub fn create_new_page(app: &app::Handle, mode: CreateMode) -> page_store::Id {
    use gtk::prelude::*;

    log_debug!("creating new page ({:?})", mode);

    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();
    let id = app.get_active()
        .expect("active page during new child creation");
    let parent_id = page_store.get_parent(id);

    let new_id = page_store.insert({
        let data = page_store::InsertData::new("about:blank".into());
        match mode {
            CreateMode::Sibling => data
                .with_parent(parent_id)
                .with_position(page_store::InsertPosition::After(id)),
            CreateMode::Child => data
                .with_parent(Some(id)),
        }
    }).expect("new created page");

    page_tree_view.select(new_id);

    let navigation_bar = app.navigation_bar();
    navigation_bar.address_entry().grab_focus();

    new_id
}

fn reopen(app: &app::Handle, id: Option<page_store::Id>) {
    log_debug!("reopen from recent: {:?}", id);

    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();
    let page = match id {
        Some(id) => unwrap_or_return!(page_store.recently_closed_state().pull(id)),
        None => unwrap_or_return!(page_store.recently_closed_state().pull_most_recent()),
    };

    let mut parent = (None, page_store::InsertPosition::Start);
    for position in page.position.iter() {
        if let Some(parent_id) = position.0 {
            if page_store.exists(parent_id) {
                parent = (Some(parent_id), page_store::InsertPosition::At(position.1));
                break;
            }
        } else {
            parent = (None, page_store::InsertPosition::At(position.1));
            break;
        }
    }

    let recently_closed::Page { title, uri, id, .. } = page;

    page_store.insert(
        page_store::InsertData::new(uri)
            .with_title(title)
            .with_parent(parent.0)
            .with_position(parent.1)
            .with_reused_id(id)
    ).expect("reopened page");

    page_tree_view.select(id);
}

fn adjust_for_load_state(app: &app::Handle, state: page_store::LoadState) {
    use gio::prelude::*;

    let app_actions = app.app_actions();

    app_actions.go_back_action.set_enabled(state.can_go_back);
    app_actions.go_forward_action.set_enabled(state.can_go_forward);

    app_actions.reload_action.set_enabled(!state.is_loading);
    app_actions.reload_bp_action.set_enabled(!state.is_loading);
    app_actions.stop_loading_action.set_enabled(state.is_loading);
}

fn confirm_close_children(app: &app::Handle, count: i32) -> Option<bool> {

    const CLOSE_ALL: i32 = 1;
    const CLOSE_ONE: i32 = 2;
    const CANCEL: i32 = 3;
    
    let window = app.window();

    let answer = window::confirm_action(
        &window,
        &format!("Do you want to close {} {}?",
            count,
            if count == 1 { "page" } else { "pages" },
        ),
        &[
            ("Close Current", CLOSE_ONE),
            ("Close All", CLOSE_ALL),
            ("Cancel", CANCEL),
        ],
        CLOSE_ONE,
    );
    match answer {
        CLOSE_ALL => Some(true),
        CLOSE_ONE => Some(false),
        _ => None,
    }
}

fn find_next_selection(app: &app::Handle, parent: Option<page_store::Id>, position: u32)
-> page_store::Id {
    
    let page_store = app.page_store();

    if let Some(id) = page_store.find_previous(parent, position) { id }
    else if let Some(id) = page_store.find_next_incl(parent, position + 1) { id }
    else {
        page_store.insert(
            page_store::InsertData::new("about:blank".into())
                .with_title(Some("about:blank".into()))
                .with_position(page_store::InsertPosition::End)
        ).expect("created fallback page to have something to select")
    }
}

pub fn try_close_page(app: &app::Handle, id: page_store::Id) {
    use gtk::prelude::*;

    log_debug!("closing page {}", id);

    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();

    let close_children =
        if let Some(count) = page_store.has_children(id) {
            unwrap_or_return!(confirm_close_children(app, count + 1))
        } else {
            false
        };

    let active_id = app.get_active();

    let position =
        if Some(id) == active_id {
            page_store.position(id)
        } else {
            None
        };

    let select = position.map(|(parent, position)| find_next_selection(app, parent, position));

    app.without_select(|| page_store.close(id, close_children));
    
    if let Some(select) = select {
        page_tree_view.widget().get_selection().unselect_all();
        page_tree_view.select(select);
    }
}
