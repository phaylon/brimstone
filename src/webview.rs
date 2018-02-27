
use webkit2gtk;
use gdk;

use app;
use page_store;
use mouse;
use window;
use text;

fn on_property_uri_notify(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
) {
    use webkit2gtk::{ WebViewExt };
    use gtk::{ EntryExt };

    let uri = view.get_uri();

    let page_store = try_extract!(app.page_store());
    let nav_bar = try_extract!(app.navigation_bar());
    let history = try_extract!(app.history());

    log_debug!("uri for {} now {:?}", id, uri);

    if let &Some(ref uri) = &uri {
        expect_ok!(history.update_access(&uri), "history update", id, uri);
    }

    let uri = uri.unwrap_or_else(|| "".into());
    if app.is_active(id) {
        nav_bar.address_entry().set_text(&uri);
    }
    page_store.set_uri(id, uri.into());
}

fn on_property_title_notify(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
) {
    use webkit2gtk::{ WebViewExt };

    let title = view.get_title();
    let uri = view.get_uri();

    let page_store = try_extract!(app.page_store());
    let history = try_extract!(app.history());

    log_debug!("title for {} now {:?}", id, title);

    if let (&Some(ref uri), &Some(ref title)) = (&uri, &title) {
        expect_ok!(history.update_title(&uri, &title), "successful history update", id, title);
    }
    page_store.set_title(id, title.clone().map(|s| s.into()));
    if app.is_active(id) {
        window::set_title(
            app,
            title.as_ref().map(|val| val.as_str()),
            uri.as_ref().map(|val| val.as_str()),
        );
    } else {
        page_store.set_unread(id);
    }
}

#[derive(Debug)]
enum PolicyDecision {
    Navigation {
        decision: webkit2gtk::NavigationPolicyDecision,
        navigation_type: webkit2gtk::NavigationType,
        button: u32,
        shift_modifier: bool,
    },
    NewWindow {
        decision: webkit2gtk::NavigationPolicyDecision,
    },
    Response {
        decision: webkit2gtk::ResponsePolicyDecision,
    },
}

fn categorize_policy_decision(
    decision: &webkit2gtk::PolicyDecision,
    decision_type: webkit2gtk::PolicyDecisionType,
) -> Option<PolicyDecision> {
    use gtk::{ Cast };
    use webkit2gtk::{ NavigationPolicyDecisionExt };

    match decision_type {
        webkit2gtk::PolicyDecisionType::NavigationAction => {
            let decision = decision.clone().downcast::<webkit2gtk::NavigationPolicyDecision>()
                .expect("policy decision castable to navigation specific type");
            let navigation_type = decision.get_navigation_type();
            let button = decision.get_mouse_button();
            let shift_modifier = decision.get_navigation_action()
                .and_then(|action| gdk::ModifierType::from_bits(action.get_modifiers()))
                .map(|modifiers| modifiers.contains(gdk::ModifierType::SHIFT_MASK))
                .unwrap_or(false);
            Some(PolicyDecision::Navigation { decision, navigation_type, button, shift_modifier })
        },
        webkit2gtk::PolicyDecisionType::NewWindowAction => {
            let decision = decision.clone().downcast::<webkit2gtk::NavigationPolicyDecision>()
                .expect("policy decision castable to navigation specific type");
            Some(PolicyDecision::NewWindow { decision })
        },
        webkit2gtk::PolicyDecisionType::Response => {
            let decision = decision.clone().downcast::<webkit2gtk::ResponsePolicyDecision>()
                .expect("policy decision castable to response specific type");
            Some(PolicyDecision::Response { decision })
        },
        _ => None,
    }
}

fn open_child_page(
    app: &app::Handle,
    uri: text::RcString,
    parent: page_store::Id,
    select: bool,
) {
    let page_store = try_extract!(app.page_store());

    let new_id = expect_some!(page_store.insert(
        page_store::InsertData::new(uri.clone())
            .with_title(Some(uri))
            .with_parent(Some(parent))
    ), "child page creation", parent);

    let page_tree_view = try_extract!(app.page_tree_view());
    page_tree_view.expand(parent, false);

    if select {
        page_tree_view.select(new_id);
    }
}

fn on_decide_policy(
    app: &app::Handle,
    id: page_store::Id,
    _view: &webkit2gtk::WebView,
    decision: &webkit2gtk::PolicyDecision,
    decision_type: webkit2gtk::PolicyDecisionType,
) -> bool {
    use webkit2gtk::{ NavigationPolicyDecisionExt, PolicyDecisionExt, URIRequestExt };

    match categorize_policy_decision(decision, decision_type) {
        Some(PolicyDecision::NewWindow { decision }) => {
            let req = try_or_false!(decision.get_request());
            let uri = try_or_false!(req.get_uri());
            decision.ignore();
            open_child_page(app, uri.into(), id, false);
            true
        },
        Some(PolicyDecision::Navigation {
            navigation_type: webkit2gtk::NavigationType::LinkClicked,
            button: mouse::BUTTON_MIDDLE,
            decision,
            shift_modifier,
        }) => {
            let req = try_or_false!(decision.get_request());
            let uri = try_or_false!(req.get_uri());
            decision.ignore();
            open_child_page(app, uri.into(), id, shift_modifier);
            true
        },
        _ => false,
    }
}

fn on_load_changed(
    app: &app::Handle,
    id: page_store::Id,
    view: &webkit2gtk::WebView,
    event: webkit2gtk::LoadEvent,
) {
    use webkit2gtk::{ WebViewExt };
    use gio::{ TlsCertificateExt };

    let page_store = try_extract!(app.page_store());

    let is_loading = view.is_loading();

    let tls_state = if !is_loading {
        match view.get_tls_info() {
            None => page_store::TlsState::Insecure,
            Some((cert, flags)) => if flags.is_empty() {
                if cert.get_issuer().is_some() {
                    page_store::TlsState::Encrypted
                } else {
                    page_store::TlsState::SelfSigned
                }
            } else {
                page_store::TlsState::Insecure
            },
        }
    } else {
        page_store::TlsState::Insecure
    };

    let state = page_store::LoadState {
        can_go_forward: view.can_go_forward(),
        can_go_back: view.can_go_back(),
        event: Some(event),
        is_loading,
        tls_state,
    };

    log_trace!("load state for {}: {:?}", id, state);

    page_store.set_load_state(id, state);
}

fn on_mouse_target_changed(
    app: &app::Handle,
    _id: page_store::Id,
    _view: &webkit2gtk::WebView,
    hit: &webkit2gtk::HitTestResult,
) {
    use webkit2gtk::{ HitTestResultExt };

    let status_bar = try_extract!(app.status_bar());
    status_bar.set_hover_uri(hit.get_link_uri());
}

pub fn create(id: page_store::Id, app: &app::Handle) -> webkit2gtk::WebView {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };

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

    new_view.connect_load_changed(with_cloned!(app, move |view, change| {
        on_load_changed(&app, id, view, change);
    }));

    new_view
}

pub fn setup(app: &app::Handle) {
    use gtk::{ WidgetExt, BoxExt };

    let page_tree_view = try_extract!(app.page_tree_view());
    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {
        log_debug!("showing webview for page {}", id);
        let view_space = try_extract!(app.view_space());
        let page_store = try_extract!(app.page_store());
        let view = try_extract!(page_store.get_view(id, &app));
        match app.active_webview() {
            Some(webview) => webview.hide(),
            None => (),
        }
        app.set_active(id, view.clone());
        if view.get_parent().is_none() {
            view_space.pack_start(&view, true, true, 0);
        }
        view_space.show();
        view.show_all();
    }));
}

pub fn create_web_context(is_private: bool) -> webkit2gtk::WebContext {
    use webkit2gtk::{ WebContextExt };
    use gtk::{ ToVariant };

    let web_context =
        if is_private {
            webkit2gtk::WebContext::new_ephemeral()
        } else {
            webkit2gtk::WebContext::get_default().expect("default web context")
        };

    web_context.set_web_extensions_initialization_user_data(&"webkit".to_variant());
    web_context.set_web_extensions_directory("target-we/debug");
    web_context.set_tls_errors_policy(webkit2gtk::TLSErrorsPolicy::Fail);
    web_context.set_process_model(webkit2gtk::ProcessModel::MultipleSecondaryProcesses);
    web_context.set_web_process_count_limit(1);

    web_context
}

pub fn create_user_content_manager() -> webkit2gtk::UserContentManager {

    let user_content_manager = webkit2gtk::UserContentManager::new();
    user_content_manager
}
