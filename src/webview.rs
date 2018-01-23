
use app;
use webkit2gtk;
use page_store;
use page_tree_store;

pub fn create(id: page_store::Id, data: &app::Data) -> webkit2gtk::WebView {
    use webkit2gtk::{ WebViewExtManual, WebViewExt };
    use gtk::{ GtkWindowExt, EntryExt };

    let new_view = webkit2gtk::WebView::new_with_context_and_user_content_manager(
        &data.web_context,
        &data.user_content_manager,
    );

    let page_store = data.page_store.clone();
    let page_tree_store = data.page_tree_store.clone();
    let window = data.window.clone();
    let active_id = data.active_page_store_id.clone();
    let nav_bar = data.navigation_bar.clone();

    let url_page_store = page_store.clone();
    new_view.connect_property_uri_notify(move |view| {
        let url = view.get_uri().unwrap_or_else(|| "".into());

        url_page_store.set_url_for(id, url.clone());
        nav_bar.address_entry.set_text(&url);
    });

    new_view.connect_property_title_notify(move |view| {
        let title = view.get_title();

        page_store.set_title_for(id, title.clone());
        if let Some(iter) = page_tree_store::find_iter_by_id(&page_tree_store, id) {
            page_tree_store::set::title(&page_tree_store, &iter, match title {
                Some(ref title) => title.clone(),
                None => "<Unnamed>".into(),
            });
        }
        if let Some(active_id) = active_id.get() {
            if active_id == id {
                match title {
                    Some(ref title) => window.set_title(&format!("{} - Brimstone", &title)),
                    None => window.set_title("Brimstone"),
                }
            }
        }
    });

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
