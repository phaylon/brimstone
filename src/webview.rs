
use webkit2gtk;
use gdk;

use app;
use page_store;
use action;
use mouse;
use page_tree_view;

fn on_property_uri_notify(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
) {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };
    use gtk::{ EntryExt, Cast };

    let uri = view.get_uri().unwrap_or_else(|| "".into());

    let page_store = try_extract!(app.page_store());
    let nav_bar = try_extract!(app.navigation_bar());

    if app.is_active(id) {
        nav_bar.address_entry().set_text(&uri);
    }

    page_store.set_uri(id, uri);
}

fn on_property_title_notify(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
) {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };
    use gtk::{ EntryExt, Cast };

    let title = view.get_title();
    let uri = view.get_uri();

    let page_store = try_extract!(app.page_store());

    page_store.set_title(id, title.clone());
    if app.is_active(id) {
        app.perform(action::window::SetTitle {
            title: title.as_ref().map(|title| &title[..]),
            uri: uri.as_ref().map(|uri| &uri[..]).unwrap_or(""),
        });
    }
}

fn on_decide_policy(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
    pol_decision: &webkit2gtk::PolicyDecision,
    pol_type: webkit2gtk::PolicyDecisionType,
) -> bool {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };
    use gtk::{ EntryExt, Cast };
    use webkit2gtk::{ NavigationPolicyDecisionExt, PolicyDecisionExt, URIRequestExt };

    fn handle_middle_click(
        app: &app::Handle,
        id: page_store::Id,
        view: &webkit2gtk::WebView,
        pol_decision: &webkit2gtk::PolicyDecision,
        pol_type: webkit2gtk::PolicyDecisionType,
    ) -> bool {

        if pol_type != webkit2gtk::PolicyDecisionType::NavigationAction {
            return false;
        }
        let nav_pol_decision =
            match pol_decision.clone().downcast::<webkit2gtk::NavigationPolicyDecision>() {
                Ok(casted) => casted,
                Err(_) => return false,
            };
        if nav_pol_decision.get_navigation_type() != webkit2gtk::NavigationType::LinkClicked {
            return false;
        }
        if nav_pol_decision.get_mouse_button() != mouse::BUTTON_MIDDLE {
            return false;
        }
        let req = match nav_pol_decision.get_request() {
            Some(req) => req,
            None => return false,
        };
        let uri = match req.get_uri() {
            Some(uri) => uri,
            None => return false,
        };

        let select = match nav_pol_decision.get_navigation_action() {
            None => false,
            Some(nav_action) => match gdk::ModifierType::from_bits(nav_action.get_modifiers()) {
                Some(modifiers) => modifiers.contains(gdk::ModifierType::SHIFT_MASK),
                none => false,
            },
        };

        pol_decision.ignore();
        // TODO related to current webview
        let new_id = app.perform(action::page::Create {
            uri: uri.clone(),
            title: Some(uri),
            parent: Some(id),
            position: page_store::InsertPosition::Start,
        }).expect("page creation");
        let page_tree_store = try_extract!(app.page_tree_store());
        let page_tree_view = try_extract!(app.page_tree_view());
        page_tree_view::expand_id(&page_tree_store, &page_tree_view, id, false);

        if select {
            page_tree_view::select_id(&page_tree_store, &page_tree_view, new_id);
        }

        true
    }

    if handle_middle_click(app, id, view, pol_decision, pol_type) {
        true
    } else {
        false
    }
}

fn on_load_changed(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
) {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };

    let state = page_store::LoadState {
        can_go_forward: view.can_go_forward(),
        can_go_back: view.can_go_back(),
        is_loading: view.is_loading(),
    };
    app.perform(action::page::LoadStateChange {
        id,
        state,
    });
}

fn on_mouse_target_changed(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
    hit: &webkit2gtk::HitTestResult,
) {
    use webkit2gtk::{ HitTestResultExt };

    let status_bar = try_extract!(app.status_bar());
    status_bar.set_hover_uri(hit.get_link_uri());
}

pub fn create(id: page_store::Id, app: &app::Handle) -> webkit2gtk::WebView {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };
    use gtk::{ EntryExt, Cast };

    let new_view = webkit2gtk::WebView::new_with_context_and_user_content_manager(
        &try_extract!(app.web_context()),
        &try_extract!(app.user_content_manager()),
    );

    new_view.connect_property_uri_notify(with_cloned!(app, move |view| {
        on_property_uri_notify(&app, id, view);
    }));

    new_view.connect_property_title_notify(with_cloned!(app, move |view| {
        on_property_title_notify(&app, id, view);
    }));

    new_view.connect_decide_policy(with_cloned!(app, move |view, pol_decision, pol_type| {
        on_decide_policy(&app, id, view, pol_decision, pol_type)
    }));

    new_view.connect_mouse_target_changed(with_cloned!(app, move |view, hit, _| {
        on_mouse_target_changed(&app, id, view, hit);
    }));

    new_view.connect_load_changed(with_cloned!(app, move |view, _change| {
        on_load_changed(&app, id, view);
    }));

    new_view
}

pub fn create_web_context() -> webkit2gtk::WebContext {
    use webkit2gtk::{ WebContextExt };
    use gtk::{ ToVariant };

    let web_context = webkit2gtk::WebContext::get_default().unwrap();
    web_context.set_web_extensions_initialization_user_data(&"webkit".to_variant());
    web_context.set_web_extensions_directory("../webkit2gtk-webextension-rs/example/target/debug/");

    web_context
}

pub fn create_user_content_manager() -> webkit2gtk::UserContentManager {

    let user_content_manager = webkit2gtk::UserContentManager::new();
    user_content_manager
}
