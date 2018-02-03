
use gtk;
use pango;

use app;
use page_tree_store;
use action;
use page_store;
use mouse;
use page_context_menu;

pub fn select_id(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id) {
    use gtk::{ TreeViewExt, TreeSelectionExt, TreeModelExt };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    let path = try_extract!(store.get_path(&iter));
    view.expand_to_path(&path);
    view.get_selection().select_iter(&iter);
}

pub fn collapse_id(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id) {
    use gtk::{ TreeViewExt, TreeModelExt, Cast };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    let model: gtk::TreeModel = store.clone().upcast();
    let path = try_extract!(model.get_path(&iter));

    view.collapse_row(&path);
}

pub fn expand_id(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id, all: bool) {
    use gtk::{ TreeViewExt, TreeModelExt, Cast };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    let model: gtk::TreeModel = store.clone().upcast();
    let path = try_extract!(model.get_path(&iter));

    view.expand_row(&path, all);
}

pub fn is_expanded(store: &gtk::TreeStore, view: &gtk::TreeView, id: page_store::Id) -> bool {
    use gtk::{ TreeViewExt, TreeModelExt, Cast };

    let iter = try_extract!(page_tree_store::find_iter_by_id(store, id));
    let model: gtk::TreeModel = store.clone().upcast();
    let path = try_extract!(model.get_path(&iter));

    view.row_expanded(&path)
}

pub fn create() -> gtk::TreeView {
    use gtk::{
        TreeViewExt, CellLayoutExt, TreeSelectionExt, CellRendererTextExt, TreeViewColumnExt,
    };

    let title_column = {
        let title_column = gtk::TreeViewColumn::new();
        let title_cell = gtk::CellRendererText::new();
        title_cell.set_property_ellipsize(pango::EllipsizeMode::End);
        title_column.pack_start(&title_cell, true);
        title_column.add_attribute(&title_cell,
            "text", page_tree_store::index::title as i32);
        title_column.add_attribute(&title_cell,
            "style", page_tree_store::index::style as i32);
        title_column.add_attribute(&title_cell,
            "weight", page_tree_store::index::weight as i32);
        title_column.add_attribute(&title_cell,
            "underline", page_tree_store::index::is_pinned as i32);
        title_column.set_expand(true);
        title_column
    };

    let children_column = {
        let children_column = gtk::TreeViewColumn::new();
        let children_cell = gtk::CellRendererText::new();
        children_column.pack_end(&children_cell, false);
        children_column.add_attribute(&children_cell,
            "text", page_tree_store::index::child_info as i32);
        children_column.add_attribute(&children_cell,
            "visible", page_tree_store::index::has_children as i32);
        children_column
    };

    let view = gtk::TreeView::new();
    view.append_column(&title_column);
    view.append_column(&children_column);
    view.set_expander_column(&title_column);
    view.set_tooltip_column(page_tree_store::index::title as i32);
    view.set_enable_tree_lines(true);
    view.set_headers_visible(false);
    view.set_show_expanders(true);
    view.set_reorderable(true);
    view.get_selection().set_mode(gtk::SelectionMode::Single);

    view
}

pub fn setup(app: app::Handle) {
    use gtk::{ TreeViewExt, TreeSelectionExt, WidgetExt };

    let page_tree_view = app.page_tree_view().unwrap();
    page_tree_view.set_model(&app.page_tree_store().unwrap());

    page_tree_view.connect_drag_begin(with_cloned!(app, move |_view, _| {
        app.set_select_ignored(true);
    }));
    page_tree_view.connect_drag_end(with_cloned!(app, move |view, _| {
        app.set_select_ignored(false);
        let store = try_extract!(app.page_tree_store());
        let id = try_extract!(app.get_active());
        select_id(&store, view, id);
    }));

    page_tree_view.get_selection().connect_changed(with_cloned!(app, move |selection| {
        let (model, iter) = match selection.get_selected() {
            None => return,
            Some(selected) => selected,
        };
        let id = page_tree_store::get::id(&model, &iter);
        app.perform(action::page::Select { id });
    }));

    page_tree_view.connect_drag_end(with_cloned!(app, move |view, _| {
        use gtk::{ Cast };

        let page_store = try_extract!(app.page_store());
        let session = try_extract!(app.session_updater());
        let count = page_store.pinned_count();
        if count == 0 {
            return;
        }
        let page_tree_store = page_store.tree_store();
        let model = page_tree_store.clone().upcast();
        let mut seen = 0;
        let mut misplaced = Vec::new();
        let mut last_position = 0;
        for (child_id, child_iter) in page_store.children(None) {
            if page_tree_store::get::is_pinned(&model, &child_iter) {
                seen += 1;
                if seen == count {
                    break;
                }
            } else {
                misplaced.push(child_id);
            }
            last_position += 1;
        }
        last_position += 1;
        for child_id in misplaced {
            page_store.move_to(child_id, None, last_position);
        }
        session.update_tree(&page_tree_store);
        if let Some(id) = app.get_active() {
            select_id(&page_tree_store, view, id);
        }
    }));

    page_tree_view.connect_button_press_event(with_cloned!(app, move |view, event| {
        use gtk::{ TreeModelExt, Cast, MenuExtManual, ToVariant };
        use gio::{ SimpleActionExt };
        use page_tree_view;
        
        let (x, y) = event.get_position();
                
        let path = match view.get_path_at_pos(x as _, y as _) {
            Some((Some(path), _, _, _)) => path,
            _ => return gtk::prelude::Inhibit(false),
        };
        
        if event.get_button() == mouse::BUTTON_RIGHT {
            (||{
                let store = try_extract!(app.page_tree_store());
                let iter = try_extract!(view.get_model().unwrap().get_iter(&path));
                let page_store = try_extract!(app.page_store());
                let model = store.clone().upcast();
                let id = page_tree_store::get::id(&model, &iter);

                app.set_page_tree_target(Some(id));

                let map = try_extract!(app.page_context_menu());
                let has_children = page_store.has_children(id).is_some();
                let is_pinned = page_store.get_pinned(id);
                let is_expanded = page_tree_view::is_expanded(&store, view, id);

                map.update_state(page_context_menu::State {
                    has_children,
                    is_expanded,
                    is_pinned,
                });
                map.menu().popup_easy(event.get_button(), event.get_time());
            })();
            gtk::prelude::Inhibit(true)
        } else {
            (||{
                let store = try_extract!(app.page_tree_store());
                let iter = try_extract!(view.get_model().unwrap().get_iter(&path));
                let page_store = try_extract!(app.page_store());
                let model = store.clone().upcast();
                let id = page_tree_store::get::id(&model, &iter);
                let is_pinned = page_store.get_pinned(id);
                view.set_reorderable(!is_pinned);
            })();
            gtk::prelude::Inhibit(false)
        }
    }));
}

