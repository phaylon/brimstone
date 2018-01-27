
macro_rules! try_extract {
    ($src:expr) => { match $src { Some(value) => value, None => return Default::default() } }
}

macro_rules! with_cloned {
    ($var:ident, $body:expr $(,)*) => {
        with_cloned!($var, $var, $body);
    };
    ($var:ident, $src:expr, $body:expr $(,)*) => {{
        let $var = $src.clone();
        $body
    }}
}

macro_rules! consts_seq {
    ($ty:ty, $index:expr, $name:ident $($rest:tt)*) => {

        #[allow(non_upper_case_globals)]
        pub const $name: $ty = $index;

        consts_seq!($ty, $index + 1 $($rest)*);
    };
    ($ty:ty, $index:expr $(,)*) => {}
}

macro_rules! mod_tree_store {
    (   $name:ident:
        struct { $($fname:ident: $ftype:ty),* $(,)* }
        $($rest:tt)*
    ) => {

        pub mod $name {
            use gtk;

            pub mod index {
                consts_seq!(u32, 0, $($fname),*);
            }

            pub mod set {
                $(
                    pub fn $fname(store: &::gtk::TreeStore, iter: &::gtk::TreeIter, val: $ftype) {
                        use ::gtk::{ TreeStoreExtManual, ToValue };
                        store.set_value(iter, super::index::$fname, &val.to_value());
                    }
                )*
            }

            pub mod get {
                $(
                    pub fn $fname(store: &::gtk::TreeModel, iter: &::gtk::TreeIter) -> $ftype {
                        use ::gtk::{ TreeModelExt };
                        store.get_value(iter, super::index::$fname as i32).get().unwrap()
                    }
                )*
            }

            pub struct Entry {
                $(pub $fname:$ftype),*
            }

            pub fn create() -> gtk::TreeStore {
                gtk::TreeStore::new(&[
                    $(<$ftype as gtk::StaticType>::static_type()),*
                ])
            }

            pub fn insert(
                store: &gtk::TreeStore,
                parent: Option<gtk::TreeIter>,
                position: Option<u32>,
                entry: Entry,
            ) -> gtk::TreeIter {
                use gtk::{ TreeStoreExtManual };

                store.insert_with_values(
                    parent.as_ref(),
                    position,
                    &[$(self::index::$fname),*],
                    &[$(&entry.$fname),*],
                )
            }

            $($rest)*
        }
    }
}

