
use gtk;

use app;
use scrolled;

pub fn create() -> gtk::Paned {

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned
}

pub fn setup(app: app::Handle) {
    use gtk::{ PanedExt, BoxExt, WidgetExt };
    
    let main_paned = app.main_paned().unwrap();
    
    let page_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page_box.pack_start(&app.page_bar().unwrap().container(), false, true, 0);
    page_box.pack_start(&scrolled::create(app.page_tree_view().unwrap()), true, true, 0);
    page_box.pack_start(&app.status_bar().unwrap().page_tree_status(), false, true, 0);

    let web_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    web_box.pack_start(&app.navigation_bar().unwrap().container(), false, true, 0);
    web_box.pack_start(&app.view_space().unwrap(), true, true, 0);
    web_box.pack_start(&app.status_bar().unwrap().webview_status(), false, true, 0);

    main_paned.pack1(&page_box, false, true);
    main_paned.pack2(&web_box, true, true);
    main_paned.set_position(150);

    &app.view_space().unwrap().set_name("view-space");
}

