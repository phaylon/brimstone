
use gtk;

use app;
use scrolled;
use layout;

pub fn create() -> gtk::Paned {

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned
}

pub fn setup(app: &app::Handle) {
    use gtk::{ PanedExt, WidgetExt };
    use layout::{ BuildBox, BuildPaned };
    
    let main_paned = &app.main_paned().expect("main paned during setup");

    main_paned.add1_secondary(&layout::vbox()
        .add_start(&app.page_bar().expect("page bar during setup").container())
        .add_start_fill(&scrolled::create(
            app.page_tree_view().expect("page tree during setup").widget().clone()
        ))
        .add_start(&app.status_bar().expect("status bar during setup").page_tree_status())
    );
    main_paned.add2_primary(&layout::vbox()
        .add_start(&app.navigation_bar().expect("navigation bar during setup").container())
        .add_start_fill(&layout::vpaned(None)
            .add1_primary(&app.view_space().expect("view space during setup"))
            .add2_secondary(app.stored().expect("stored view during setup").container())
        )
        .add_start(&app.status_bar().expect("status bar during setup").webview_status())
    );
    main_paned.set_position(150);

    app.view_space().expect("view space during setup").set_name("view-space");
}

