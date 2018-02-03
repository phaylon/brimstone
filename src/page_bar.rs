
use std::rc;

use gtk;

use app;
use app_action;
use bar;

pub struct Bar {
    container: gtk::Box,
    add_page_button: gtk::Button,
    remove_page_button: gtk::Button,
}

pub struct Handle {
    bar: rc::Rc<Bar>,
}

impl Handle {

    pub fn new(bar: rc::Rc<Bar>) -> Handle {
        Handle { bar }
    }

    pub fn container(&self) -> gtk::Box { self.bar.container.clone() }
}

pub fn create() -> Bar {
    Bar {
        container: bar::create_container(),
        add_page_button: bar::create_nav_button("list-add", true, true),
        remove_page_button: bar::create_nav_button("list-remove", true, true),
    }
}

pub fn setup(app: app::Handle) {
    use gtk::{ BoxExt, ActionableExt, SizeGroupExt };

    let bar = app.page_bar().unwrap().bar;
    bar.container.pack_start(&bar.add_page_button, false, true, 0);
    bar.container.pack_start(&bar.remove_page_button, false, true, 0);

    app.bar_size_group().unwrap().add_widget(&bar.container);
    
    bar.add_page_button.set_action_name(Some(app_action::ACTION_NEW));
    bar.remove_page_button.set_action_name(Some(app_action::ACTION_CLOSE));
}
