
use gtk;

use app;

const APP_NAME: &str = "Brimstone";

pub fn create(app: &gtk::Application) -> gtk::ApplicationWindow {
    use gtk::prelude::*;
    
    let window = gtk::ApplicationWindow::new(&app);
    window.set_title("Brimstone");
    window.set_default_geometry(600, 300);

    window
}

pub fn set_title(app: &app::Handle, title: Option<&str>, uri: Option<&str>) {
    use gtk::prelude::*;

    let title = title.or(uri).unwrap_or("");
    let window = app.window();

    if title.is_empty() {
        window.set_title(APP_NAME);
    } else {
        window.set_title(&format!("{} - {}", title, APP_NAME));
    }
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;

    let window = app.window();
    let page_tree_view = app.page_tree_view();

    window.add(&app.main_paned());

    window.connect_delete_event(|window, _event| {
        match confirm_close(window, "the application") {
            CloseAnswer::Close => gtk::prelude::Inhibit(false),
            CloseAnswer::Cancel => gtk::prelude::Inhibit(true),
        }
    });

    page_tree_view.on_selection_change(with_cloned!(app, move |_map, &id| {
        let page_store = app.page_store();
        let title = page_store.get_title(id);
        let uri = page_store.get_uri(id);
        set_title(
            &app,
            title.as_ref().map(|val| val.as_str()),
            uri.as_ref().map(|val| val.as_str()),
        );
    }));
}

pub fn confirm_action(
    window: &gtk::ApplicationWindow,
    text: &str,
    buttons: &[(&str, i32)],
    default: i32,
) -> i32 {
    use gtk::prelude::*;

    let dialog = gtk::MessageDialog::new(
        Some(window),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Question,
        gtk::ButtonsType::None,
        text,
    );

    for button in buttons {
        dialog.add_button(button.0, button.1);
    }
    dialog.set_default_response(default);

    let result = dialog.run();

    dialog.destroy();

    result
}

pub enum CloseAnswer { Close, Cancel }

pub fn confirm_close(window: &gtk::ApplicationWindow, what: &str) -> CloseAnswer {

    const CLOSE: i32 = 2;
    const CANCEL: i32 = 3;

    let result = confirm_action(
        window,
        &format!("Do you really want to close {}?", what),
        &[("Close", CLOSE), ("Cancel", CANCEL)],
        CLOSE,
    );
    match result {
        CLOSE => CloseAnswer::Close,
        _ => CloseAnswer::Cancel,
    }
}

pub fn present(app: &app::Handle) {
    use gtk::prelude::*;

    app.window().show_all();
}
