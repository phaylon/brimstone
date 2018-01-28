
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

pub enum CloseAnswer { Close, Cancel }

pub fn confirm_close(window: &gtk::ApplicationWindow, what: &str) -> CloseAnswer {
    use gtk::{ DialogExt, WidgetExt };

    let dialog = gtk::MessageDialog::new(
        Some(window),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        gtk::MessageType::Question,
        gtk::ButtonsType::None,
        &format!("Do you really want to close {}?", what),
    );

    const CLOSE: i32 = 2;
    const CANCEL: i32 = 3;

    dialog.add_button("Close", CLOSE);
    dialog.add_button("Cancel", CANCEL);
    dialog.set_default_response(CLOSE);

    let delete_id: i32 = gtk::ResponseType::DeleteEvent.into();
    let result = match dialog.run() {
        CLOSE => CloseAnswer::Close,
        CANCEL => CloseAnswer::Cancel,
        other if other == delete_id => CloseAnswer::Cancel,
        other => panic!("Unexpected close dialog return value: {}", other),
    };

    dialog.destroy();
    result
}

pub fn present(app: &app::Application) {
    use gtk::{ WidgetExt };

    app.handle().window().unwrap().show_all();
}
