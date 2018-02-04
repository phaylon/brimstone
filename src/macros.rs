
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

macro_rules! log_if_level {
    ($level:expr, $( ( $($arg:tt)* ) ),* $(,)*) => {
        if ::CURRENT_LOG_LEVEL.load(::std::sync::atomic::Ordering::Relaxed) >= $level {
            $(
                eprintln!("{}: {}", module_path!(), &format!($($arg)*));
            )*
        }
    }
}

macro_rules! log_trace {
    ($($arg:tt)*) => { log_if_level!(::LOG_TRACE, ($($arg)*)) }
}

macro_rules! log_debug {
    ($($arg:tt)*) => { log_if_level!(::LOG_DEBUG, ($($arg)*)) }
}

macro_rules! consts_seq {
    ($ty:ty, $index:expr, $name:ident $($rest:tt)*) => {

        #[allow(non_upper_case_globals)]
        pub const $name: $ty = $index;

        consts_seq!($ty, $index + 1 $($rest)*);
    };
    ($ty:ty, $index:expr $(,)*) => {}
}

macro_rules! fn_connect_notifier {
    ($field:ident, $connect:ident, $arg:ty) => {
        pub fn $connect<F>(&self, callback: F)
        where F: Fn(&Self, &$arg) + 'static {
            self.$field.connect(callback);
        }
    }
}

macro_rules! gen_tree_store_indices {
    ($($name:ident),* $(,)*) => {
        pub mod index {
            consts_seq!(u32, 0, $($name),*);
        }
    }
}

macro_rules! gen_tree_store_create {
    ($($ty:ty),* $(,)*) => {
        pub fn create() -> ::gtk::TreeStore {
            ::gtk::TreeStore::new(&[
                $(<$ty as ::gtk::StaticType>::static_type()),*
            ])
        }
    }
}

macro_rules! gen_tree_store_insert {
    ($($name:ident: $index:ident),* $(,)*) => {
        pub fn insert(
            store: &::gtk::TreeStore,
            parent: Option<&::gtk::TreeIter>,
            position: Option<u32>,
            entry: Entry,
        ) -> ::gtk::TreeIter {
            use ::gtk::{ TreeStoreExtManual };

            store.insert_with_values(
                parent,
                position,
                &[$(self::index::$index),*],
                &[$(&entry.$name),*],
            )
        }
    }
}

macro_rules! gen_tree_store_getter {
    ($name:ident, $index:ident, $ty:ty) => {
        pub fn $name<S>(store: &S, iter: &::gtk::TreeIter) -> $ty
        where S: ::gtk::IsA<::gtk::TreeModel> + ::gtk::TreeModelExt {
            store.get_value(iter, self::index::$index as i32).get().unwrap()
        }
    }
}

macro_rules! gen_tree_store_setter {
    ($name:ident, $index:ident, $ty:ty) => {
        pub fn $name(store: &::gtk::TreeStore, iter: &::gtk::TreeIter, value: $ty) {
            use ::gtk::{ TreeStoreExtManual, ToValue };
            store.set_value(iter, self::index::$index, &value.to_value());
        }
    }
}

macro_rules! gen_tree_store_entry {
    ($($name:ident: $settype:ty),* $(,)*) => {
        pub struct Entry {
            $(pub $name: $settype,)*
        }
    }
}

macro_rules! gen_tree_store_accessors {
    ($index:ident, set($set:ident: $settype:ty) $(,)*) => {
        gen_tree_store_setter!($set, $index, $settype);
    };
    ($index:ident, get($get:ident: $gettype:ty), set($set:ident: $settype:ty) $(,)*) => {
        gen_tree_store_getter!($get, $index, $gettype);
        gen_tree_store_setter!($set, $index, $settype);
    };
    ($index:ident, set($set:ident: $settype:ty), get($get:ident: $gettype:ty) $(,)*) => {
        gen_tree_store_getter!($get, $index, $gettype);
        gen_tree_store_setter!($set, $index, $settype);
    }
}

macro_rules! gen_tree_store {
    ($(( $name:ident: $ty:ty, $index:ident, $($rest:tt)*)),* $(,)*) => {
        gen_tree_store_indices!($($index),*);
        gen_tree_store_entry!($($name: $ty),*);
        gen_tree_store_create!($($ty),*);
        gen_tree_store_insert!($($name: $index),*);
        $(
            gen_tree_store_accessors!($index, $($rest)*);
        )*
    }
}

