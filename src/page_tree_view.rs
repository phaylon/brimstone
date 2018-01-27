
use gtk;
use pango;

use app;
use page_tree_store;
use action;
use page_store;

pub fn select_id(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id) {
    use gtk::{ TreeViewExt, TreeSelectionExt };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    view.get_selection().select_iter(&iter);
}

pub fn expand_id(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id, all: bool) {
    use gtk::{ TreeViewExt, TreeModelExt, Cast };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    let model: gtk::TreeModel = store.clone().upcast();
    let path = try_extract!(model.get_path(&iter));

    view.expand_row(&path, all);
}

pub fn create() -> gtk::TreeView {
    use gtk::{ TreeViewExt, CellLayoutExt, TreeSelectionExt, CellRendererTextExt };

    let view = gtk::TreeView::new();
    view.set_enable_tree_lines(true);
    view.set_headers_visible(false);
    view.set_show_expanders(true);

    let title_column = gtk::TreeViewColumn::new();
    let title_cell = gtk::CellRendererText::new();
    let favicon_cell = gtk::CellRendererPixbuf::new();
    title_cell.set_property_ellipsize(pango::EllipsizeMode::End);
    title_column.pack_start(&favicon_cell, true);
    title_column.pack_start(&title_cell, true);
    title_column.add_attribute(&title_cell, "text", page_tree_store::index::title as i32);

    view.append_column(&title_column);
    view.set_tooltip_column(page_tree_store::index::title as i32);
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

