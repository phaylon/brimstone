
use std::cell;

use gtk;
use gdk;
use pango;

use app;
use page_tree_store;
use page_store;
use mouse;
use page_context_menu;
use signal;
use app_action;

pub struct Map {
    widget: gtk::TreeView,
    selection_change_notifier: signal::Notifier<Map, page_store::Id>,
    page_tree_store: cell::RefCell<Option<gtk::TreeStore>>,
}

impl Map {

    fn_connect_notifier!(selection_change_notifier, on_selection_change, page_store::Id);

    pub fn new() -> Map {
        Map {
            widget: create_tree_view(),
            selection_change_notifier: signal::Notifier::new(),
            page_tree_store: cell::RefCell::new(None),
        }
    }

    pub fn widget(&self) -> &gtk::TreeView { &self.widget }

    pub fn page_tree_store(&self) -> gtk::TreeStore {
        let store = self.page_tree_store.borrow();
        let store = store.as_ref().expect("page_tree_store for page_tree_view");
        store.clone()
    }

    pub fn set_page_tree_store(&self, store: &gtk::TreeStore) {
        use gtk::prelude::*;

        self.widget.set_model(store);
        *self.page_tree_store.borrow_mut() = Some(store.clone());
    }

    pub fn select(&self, id: page_store::Id) {
        let iter =
            unwrap_or_return!(page_tree_store::find_iter_by_id(&self.page_tree_store(), id));
        self.select_iter(&iter);
    }

    pub fn select_iter(&self, iter: &gtk::TreeIter) {
        use gtk::prelude::*;

        let path = unwrap_or_return!(self.page_tree_store().get_path(iter));
        self.widget.expand_to_path(&path);
        self.widget.get_selection().select_iter(&iter);
    }

    pub fn select_first(&self) {
        use gtk::prelude::*;

        let iter = unwrap_or_return!(self.page_tree_store().iter_nth_child(None, 0));
        self.select_iter(&iter);
    }

    pub fn collapse(&self, id: page_store::Id) {
        let iter =
            unwrap_or_return!(page_tree_store::find_iter_by_id(&self.page_tree_store(), id));
        self.collapse_iter(&iter);
    }

    pub fn collapse_iter(&self, iter: &gtk::TreeIter) {
        use gtk::prelude::*;
        
        let path = unwrap_or_return!(self.page_tree_store().get_path(iter));
        self.widget.collapse_row(&path);
    }

    pub fn expand(&self, id: page_store::Id, all: bool) {
        let iter =
            unwrap_or_return!(page_tree_store::find_iter_by_id(&self.page_tree_store(), id));
        self.expand_iter(&iter, all);
    }

    pub fn expand_iter(&self, iter: &gtk::TreeIter, all: bool) {
        use gtk::prelude::*;
        
        let path = unwrap_or_return!(self.page_tree_store().get_path(iter));
        self.widget.expand_row(&path, all);
    }

    pub fn is_expanded(&self, id: page_store::Id) -> bool {
        use gtk::prelude::*;
        
        let iter = unwrap_or_return_false!(
            page_tree_store::find_iter_by_id(&self.page_tree_store(), id)
        );
        let path = unwrap_or_return_false!(self.page_tree_store().get_path(&iter));
        self.widget.row_expanded(&path)
    }
}

pub fn create_tree_view() -> gtk::TreeView {
    use gtk::prelude::*;

    let title_column = {
        let title_column = gtk::TreeViewColumn::new();
        let title_cell = gtk::CellRendererText::new();
        title_cell.set_property_ellipsize(pango::EllipsizeMode::End);
        title_column.pack_start(&title_cell, true);
        title_column.add_attribute(&title_cell,
            "text", page_tree_store::index::TITLE as i32);
        title_column.add_attribute(&title_cell,
            "style", page_tree_store::index::STYLE as i32);
        title_column.add_attribute(&title_cell,
            "weight", page_tree_store::index::WEIGHT as i32);
        title_column.add_attribute(&title_cell,
            "underline", page_tree_store::index::IS_PINNED as i32);
        title_column.set_expand(true);
        title_column
    };

    let children_column = {
        let children_column = gtk::TreeViewColumn::new();
        let children_cell = gtk::CellRendererText::new();
        children_column.pack_end(&children_cell, false);
        children_column.add_attribute(&children_cell,
            "text", page_tree_store::index::CHILD_INFO as i32);
        children_column.add_attribute(&children_cell,
            "visible", page_tree_store::index::HAS_CHILDREN as i32);
        children_column
    };

    let view = gtk::TreeView::new();
    view.append_column(&title_column);
    view.append_column(&children_column);
    view.set_expander_column(&title_column);
    view.set_tooltip_column(page_tree_store::index::TITLE as i32);
    view.set_enable_tree_lines(true);
    view.set_headers_visible(false);
    view.set_show_expanders(true);
    view.set_reorderable(true);
    view.get_selection().set_mode(gtk::SelectionMode::Single);

    view
}

pub fn setup(app: &app::Handle) {
    use gtk::prelude::*;

    let map = app.page_tree_view();
    let page_tree_view = map.widget();
    map.set_page_tree_store(&app.page_tree_store());

    page_tree_view.connect_drag_begin(with_cloned!(app, move |_view, _| {
        begin_drag_state(&app);
    }));

    page_tree_view.connect_drag_end(with_cloned!(app, move |_view, _| {
        end_drag_state(&app);
        correct_after_drag(&app);
        save_tree_to_session(&app);
    }));

    page_tree_view.get_selection().connect_changed(with_cloned!(app, move |selection| {
        on_selection_change(&app, selection);
    }));

    page_tree_view.connect_button_press_event(with_cloned!(app, move |view, event| {
        on_button_press(&app, view, event)
    }));
}

fn begin_drag_state(app: &app::Handle) {
    log_debug!("drag begin");
    app.set_select_ignored(true);
}

fn end_drag_state(app: &app::Handle) {
    log_debug!("drag end");
    app.set_select_ignored(false);
    let page_tree_view = app.page_tree_view();
    let id = unwrap_or_return!(app.get_active());
    page_tree_view.select(id);
}

fn on_selection_change(app: &app::Handle, selection: &gtk::TreeSelection) {
    use gtk::prelude::*;

    if app.is_select_ignored() {
        return;
    }

    let map = app.page_tree_view();
    let (model, iter) = unwrap_or_return!(selection.get_selected());
    let id = page_tree_store::get_id(&model, &iter);

    map.selection_change_notifier.emit(&map, &id);
}

fn save_tree_to_session(app: &app::Handle) {
    log_debug!("save tree to session");

    let page_store = app.page_store();
    page_store.update_session();
}

fn correct_after_drag(app: &app::Handle) {
    log_debug!("correcting tree after drag end");

    let page_store = app.page_store();
    let page_tree_view = app.page_tree_view();

    let count = page_store.pinned_count();
    if count == 0 {
        return;
    }
    let page_tree_store = page_store.tree_store();
    let mut seen = 0;
    let mut misplaced = Vec::new();
    let mut last_position = 0;
    for (child_id, child_iter) in page_store.children(None) {
        if page_tree_store::get_is_pinned(page_tree_store, &child_iter) {
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
    if let Some(id) = app.get_active() {
        page_tree_view.select(id);
    }
}

fn show_context_menu(
    app: &app::Handle,
    event: &gdk::EventButton,
    view: &gtk::TreeView,
    path: &gtk::TreePath,
) {
    use gtk::prelude::*;
    
    let store = app.page_tree_store();
    let iter = unwrap_or_return!(
        view.get_model().expect("model attached to page tree view").get_iter(&path)
    );
    let page_store = app.page_store();
    let id = page_tree_store::get_id(&store, &iter);

    app.set_page_tree_target(Some(id));

    let map = app.page_context_menu();
    let page_tree_view = app.page_tree_view();
    let has_children = page_store.has_children(id).is_some();
    let is_pinned = page_store.get_pinned(id);
    let is_expanded = page_tree_view.is_expanded(id);

    map.update_state(page_context_menu::State {
        has_children,
        is_expanded,
        is_pinned,
    });
    map.menu().popup_easy(event.get_button(), event.get_time());
}

fn close_clicked(app: &app::Handle, view: &gtk::TreeView, path: &gtk::TreePath) {
    use gtk::prelude::*;

    let store = app.page_tree_store();
    let iter = unwrap_or_return!(
        view.get_model().expect("model attached to page tree view").get_iter(&path)
    );
    let id = page_tree_store::get_id(&store, &iter);
    app_action::try_close_page(&app, id);
}

fn decide_draggable(app: &app::Handle, view: &gtk::TreeView, path: &gtk::TreePath) {
    use gtk::prelude::*;

    let store = app.page_tree_store();
    let iter = unwrap_or_return!(
        view.get_model().expect("model attached to page tree view").get_iter(&path)
    );
    let page_store = app.page_store();
    let id = page_tree_store::get_id(&store, &iter);
    let is_pinned = page_store.get_pinned(id);
    view.set_reorderable(!is_pinned);
}

fn on_button_press(app: &app::Handle, view: &gtk::TreeView, event: &gdk::EventButton)
-> gtk::prelude::Inhibit {
    use gtk::prelude::*;
    
    let (x, y) = event.get_position();
            
    let path = match view.get_path_at_pos(x as _, y as _) {
        Some((Some(path), _, _, _)) => path,
        _ => return gtk::prelude::Inhibit(false),
    };
    
    if event.get_button() == mouse::BUTTON_RIGHT {
        show_context_menu(app, event, view, &path);
        gtk::prelude::Inhibit(true)
    } else if event.get_button() == mouse::BUTTON_MIDDLE {
        close_clicked(app, view, &path);
        gtk::prelude::Inhibit(true)
    } else {
        decide_draggable(app, view, &path);
        gtk::prelude::Inhibit(false)
    }
}
