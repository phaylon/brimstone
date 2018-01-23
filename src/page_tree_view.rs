
use gtk;

use app;
use page_tree_store;
use page_store;
use action;

pub fn create() -> gtk::TreeView {
    use gtk::{ TreeViewExt, CellLayoutExt, TreeSelectionExt };

    let view = gtk::TreeView::new();
    view.set_enable_tree_lines(true);
    view.set_headers_visible(false);
    view.set_show_expanders(true);

    let title_column = gtk::TreeViewColumn::new();
    let title_cell = gtk::CellRendererText::new();
    title_column.pack_start(&title_cell, true);
    title_column.add_attribute(&title_cell, "text", page_tree_store::index::title as i32);
    view.append_column(&title_column);
    view.set_expander_column(&title_column);
    view.get_selection().set_mode(gtk::SelectionMode::Single);

    view
}

pub fn setup(app: app::Handle) {
    use gtk::{ TreeViewExt, TreeSelectionExt };

    let page_tree_view = app.page_tree_view().unwrap();
    page_tree_view.set_model(&app.page_tree_store().unwrap());

    app.with_cloned(|app| {
        page_tree_view.get_selection().connect_changed(move |selection| {
            let (model, iter) = match selection.get_selected() {
                None => return,
                Some(selected) => selected,
            };
            let id = page_tree_store::get::id(&model, &iter);
            app.perform(action::page::Select { id });
        });
    });
}

