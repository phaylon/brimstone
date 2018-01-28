
use gtk;

use app;

pub fn create(app: &gtk::Application) -> gtk::ApplicationWindow {
    use gtk::{ GtkWindowExt };
    
    let window = gtk::ApplicationWindow::new(&app);
    window.set_title("Brimstone");
    window.set_default_geometry(600, 300);

    window
}

pub fn setup(app: app::Handle) {
    use gtk::{ ContainerExt, WidgetExt };

    let window = app.window().unwrap();
    window.add(&app.main_paned().unwrap());

    window.connect_delete_event(|window, _event| {
        match confirm_close(window, "the application") {
            CloseAnswer::Close => gtk::prelude::Inhibit(false),
            CloseAnswer::Cancel => gtk::prelude::Inhibit(true),
        }
    });
}

pub fn confirm_action(
    window: &gtk::ApplicationWindow,
    text: &str,
    buttons: &[(&str, i32)],
    default: i32,
) -> i32 {
    use gtk::{ DialogExt, WidgetExt };

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

pub fn present(app: &app::Application) {
    use gtk::{ WidgetExt };

    app.handle().window().unwrap().show_all();
}
