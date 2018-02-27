
use gtk;

use app;
use app_action;
use bar;
use mouse;

pub struct Map {
    container: gtk::Box,
    add_page_button: gtk::Button,
    remove_page_button: gtk::Button,
}

impl Map {

    pub fn new() -> Map {
        Map {
            container: bar::create_container(),
            add_page_button: bar::create_nav_button("list-add", true, true),
            remove_page_button: bar::create_nav_button("list-remove", true, true),
        }
    }

    pub fn container(&self) -> gtk::Box { self.container.clone() }
}

pub fn setup(app: &app::Handle) {
    use gtk::{ BoxExt, ActionableExt, SizeGroupExt, WidgetExt };

    let bar = expect_some!(app.page_bar(), "page bar during setup");
    bar.container.pack_start(&bar.add_page_button, false, true, 0);
    bar.container.pack_start(&bar.remove_page_button, false, true, 0);

    expect_some!(app.bar_size_group(), "bar size group during setup")
        .add_widget(&bar.container);
    
    bar.add_page_button.set_action_name(Some(app_action::ACTION_NEW));
    bar.remove_page_button.set_action_name(Some(app_action::ACTION_CLOSE));

    bar.add_page_button.connect_button_release_event(with_cloned!(app, move |_button, event| {
        if event.get_button() == mouse::BUTTON_MIDDLE {
            app_action::create_new_page(&app, app_action::CreateMode::Child);
        }
        gtk::prelude::Inhibit(false)
    }));
}
