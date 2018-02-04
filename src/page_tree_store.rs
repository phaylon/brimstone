
use pango;
use gtk;

use page_store;

gen_tree_store! {
    (id: page_store::Id, ID,
        get(get_id: page_store::Id),
        set(set_id: page_store::Id),
    ),
    (title: String, TITLE,
        get(get_title: String),
        set(set_title: &str),
    ),
    (child_info: String, CHILD_INFO,
        set(set_child_info: &str),
    ),
    (has_children: bool, HAS_CHILDREN,
        get(get_has_children: bool),
        set(set_has_children: bool),
    ),
    (child_count: u32, CHILD_COUNT,
        get(get_child_count: u32),
        set(set_child_count: u32),
    ),
    (style: pango::Style, STYLE,
        get(get_style: pango::Style),
        set(set_style: pango::Style),
    ),
    (weight: i32, WEIGHT,
        get(get_weight: i32),
        set(set_weight: i32),
    ),
    (is_pinned: bool, IS_PINNED,
        get(get_is_pinned: bool),
        set(set_is_pinned: bool),
    ),
}

pub fn cmp(store: &gtk::TreeStore, a: &gtk::TreeIter, b: &gtk::TreeIter) -> bool {
    get_id(store, a) == get_id(store, b)
}

pub fn find_position(
    store: &gtk::TreeStore,
    iter: &gtk::TreeIter,
) -> Option<(Option<gtk::TreeIter>, u32)> {
    use gtk::{ TreeModelExt };

    let id = get_id(store, iter);

    let parent = store.iter_parent(iter);
    let count = store.iter_n_children(parent.as_ref());
    for index in 0..count  {
        let child = match store.iter_nth_child(parent.as_ref(), index) {
            Some(child) => child,
            None => break,
        };
        let child_id = get_id(store, &child);
        if child_id == id {
            return Some((parent, index as u32));
        }
    }
    None
}

pub fn find_iter_by_id(
    store: &gtk::TreeStore,
    id: page_store::Id,
) -> Option<gtk::TreeIter> {

    fn find_in_children(
        store: &gtk::TreeStore,
        id: page_store::Id,
        parent: Option<&gtk::TreeIter>,
    ) -> Option<gtk::TreeIter> {
        use gtk::{ TreeModelExt };

        let count = store.iter_n_children(parent);
        for index in 0..count {
            match store.iter_nth_child(parent, index) {
                Some(iter) => {
                    let iter_id = get_id(store, &iter);
                    if iter_id == id {
                        return Some(iter);
                    }
                    if let Some(iter) = find_in_children(store, id, Some(&iter)) {
                        return Some(iter);
                    }
                },
                None => (),
            }
        }
        None
    }

    find_in_children(&store, id, None)
}

