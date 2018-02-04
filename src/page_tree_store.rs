
use page_store;

gen_tree_store! {
    (id: page_store::Id, ID, get(get_id: page_store::Id), set(set_id: page_store::Id)),
}
