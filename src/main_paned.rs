
use gtk;

use app;
use scrolled;
use layout;

pub fn create() -> gtk::Paned {

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;
    use layout::{ BuildBox, BuildPaned };
    
    let main_paned = &app.main_paned();

    main_paned.add1_secondary(&layout::vbox()
        .add_start(&app.page_bar().container())
        .add_start_fill(&scrolled::create(
            app.page_tree_view().widget().clone()
        ))
        .add_start(&app.status_bar().page_tree_status())
    );
    main_paned.add2_primary(&layout::vbox()
        .add_start(&app.navigation_bar().container())
        .add_start_fill(&layout::vpaned(None)
            .add1_primary(&app.view_space())
            .add2_secondary(app.stored().container())
        )
        .add_start(&app.status_bar().webview_status())
    );
    main_paned.set_position(150);

    gtk::WidgetExt::set_name(&app.view_space(), "view-space");
}

