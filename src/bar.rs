
use gtk;

pub fn create_nav_button(name: &str, sensitivity: bool, visibility: bool) -> gtk::Button {
    use gtk::prelude::*;

    let button = gtk::Button::new_from_icon_name(
        name,
        gtk::IconSize::SmallToolbar.into(),
    );
    button.set_sensitive(sensitivity);
    button.set_visible(visibility);
    if !visibility {
        button.set_no_show_all(true);
    }
    button
}

pub fn create_address_entry() -> gtk::Entry {
    use gtk::prelude::*;

    let entry = gtk::Entry::new();
    entry.set_activates_default(false);

    entry
}

pub fn create_container() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, 5)
}

