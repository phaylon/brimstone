
use std::path;
use std::collections;
use std::cmp;

use rusqlite;
use gtk;

use text;
use page_store;
use page_tree_store;
use storage;

pub struct Node {
    id: page_store::Id,
    title: Option<text::RcString>,
    uri: text::RcString,
    is_pinned: bool,
    children: Vec<Node>,
    is_selected: bool,
}

impl Node {

    pub fn id(&self) -> page_store::Id { self.id }

    pub fn title(&self) -> Option<&text::RcString> { self.title.as_ref() }

    pub fn uri(&self) -> &text::RcString { &self.uri }

    pub fn is_pinned(&self) -> bool { self.is_pinned }

    pub fn children(&self) -> &[Node] { &self.children }

    pub fn find_selected(&self) -> Option<page_store::Id> {
        if self.is_selected {
            return Some(self.id);
        }
        for node in &self.children {
            if let Some(selected) = node.find_selected() {
                return Some(selected);
            }
        }
        None
    }
}

pub struct Tree {
    children: Vec<Node>,
}

impl Tree {

    fn from_storage(conn: &rusqlite::Connection) -> Result<Self, rusqlite::Error> {
        log_debug!("loading page tree from storage");

        fn inflate_children(
            parent_map: &mut collections::HashMap<
                Option<page_store::Id>,
                Vec<(page_store::Id, Option<text::RcString>, text::RcString, bool)>,
            >,
            parent: Option<page_store::Id>,
            selected: Option<page_store::Id>,
        ) -> Vec<Node> {
            let children = parent_map.remove(&parent).unwrap_or_else(|| Vec::new());
            let mut nodes = Vec::new();
            for (id, title, uri, is_pinned) in children {
                nodes.push(Node {
                    id, title, uri, is_pinned,
                    children: inflate_children(parent_map, Some(id), selected),
                    is_selected: Some(id) == selected,
                });
            }
            nodes
        }

        let selected: Option<page_store::Id> = conn
            .query_row("SELECT id FROM last_selected", &[], |row| row.get(0))
            .expect("last selected row exists in session storage");
        let mut stmt = conn.prepare("
            SELECT id, parent, title, uri, is_pinned
            FROM page_tree
            ORDER BY parent, position
        ")?;
        let mut rows = stmt.query(&[])?;

        let mut parent_map = collections::HashMap::new();
        while let Some(row) = rows.next() {
            let row = row?;
            let id: page_store::Id = row.get(0);
            let parent: Option<page_store::Id> = row.get(1);
            let title: Option<String> = row.get(2);
            let uri: String = row.get(3);
            let is_pinned: bool = row.get(4);
            let entry = parent_map.entry(parent).or_insert_with(|| Vec::new());
            entry.push((id, title.map(|t| t.into()), uri.into(), is_pinned));
        }

        let mut roots = inflate_children(&mut parent_map, None, selected);
        roots.sort_by_key(|node| if node.is_pinned { 0 } else { 1 });

        Ok(Tree {
            children: roots,
        })
    }

    pub fn find_selected(&self) -> Option<page_store::Id> {
        for node in &self.children {
            if let Some(selected) = node.find_selected() {
                return Some(selected);
            }
        }
        None
    }

    pub fn children(&self) -> &[Node] { &self.children }

    pub fn compact(&mut self) {
        log_debug!("compacting page tree");

        fn apply_new_ids(last_id: &mut page_store::Id, nodes: &mut [Node]) {
            for node in nodes {
                *last_id += 1;
                node.id = *last_id;
                apply_new_ids(last_id, &mut node.children);
            }
        }

        let mut last_id = 0;
        apply_new_ids(&mut last_id, &mut self.children);
    }

    pub fn find_pinned(&self) -> Vec<page_store::Id> {
        let mut pinned = Vec::new();
        for node in &self.children {
            if node.is_pinned {
                pinned.push(node.id);
            }
        }
        pinned
    }

    pub fn find_highest_id(&self) -> Option<page_store::Id> {

        fn find_highest_in_nodes(nodes: &[Node]) -> Option<page_store::Id> {
            let mut highest = None;
            for node in nodes {
                highest = Some(cmp::max(
                    highest.unwrap_or(0),
                    match find_highest_in_nodes(&node.children) {
                        Some(node_highest) => cmp::max(node.id, node_highest),
                        None => node.id,
                    },
                ));
            }
            highest
        }

        find_highest_in_nodes(&self.children)
    }
}

pub struct Session {
    storage: storage::Storage,
}

impl Session {

    pub fn open_or_create<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {
        Ok(Session {
            storage: storage::Storage::open_or_create(
                path,
                |conn| {
                    conn.execute("CREATE TABLE last_selected (id INTEGER)", &[])?;
                    conn.execute("INSERT INTO last_selected (id) VALUES (NULL)", &[])?;
                    conn.execute("
                        CREATE TABLE page_tree (
                            id INTEGER PRIMARY KEY,
                            parent INTEGER,
                            position INTEGER NOT NULL,
                            title TEXT,
                            uri TEXT,
                            is_pinned INTEGER
                        )
                    ", &[])?;
                    Ok(())
                },
                |_conn| Ok(()),
            )?,
        })
    }

    pub fn load_tree(&self) -> Result<Tree, storage::Error> {
        self.storage.with_connection(Tree::from_storage)
    }

    pub fn update_selected(&self, id: page_store::Id)
    -> Result<(), storage::Error> {
        log_debug!("updating selected page to {}", id);
        self.storage.with_connection(|conn| {
            conn.execute("UPDATE last_selected SET id = ?", &[&id])?;
            Ok(())
        })
    }

    pub fn update_node(&self, page_store: &page_store::Store, id: page_store::Id)
    -> Result<(), storage::Error> {
        log_debug!("updating page {} data", id);
                
        let data = match page_store.get_data(id) {
            Some(data) => data,
            None => return Ok(()),
        };

        self.storage.with_connection(|conn| {
            conn.execute("
                UPDATE page_tree
                SET title = ?, uri = ?, is_pinned = ?
                WHERE id = ?
            ", &[
                &data.title.as_ref().map(|s| s.as_str()),
                &data.uri.as_str(),
                &data.is_pinned,
                &id,
            ])?;
            Ok(())
        })
    }

    pub fn update_all(&self, page_store: &page_store::Store)
    -> Result<(), storage::Error> {
        use gtk::{ TreeModelExt };
        
        log_debug!("updating full tree");

        fn insert_children(
            stmt: &mut rusqlite::CachedStatement,
            page_store: &page_store::Store,
            page_tree_store: &gtk::TreeStore,
            parent: Option<&gtk::TreeIter>,
            parent_id: Option<page_store::Id>,
        ) -> Result<(), rusqlite::Error> {
            for position in 0..page_tree_store.iter_n_children(parent) {
                let iter = page_tree_store.iter_nth_child(parent, position)
                    .expect("indexed iter child is available");
                let id = page_tree_store::get_id(page_tree_store, &iter);
                let data = match page_store.get_data(id) {
                    Some(data) => data,
                    None => continue,
                };
                stmt.execute(&[
                    &id,
                    &parent_id,
                    &position,
                    &data.title.as_ref().map(|s| s.as_str()),
                    &data.uri.as_str(),
                    &data.is_pinned,
                ])?;
                insert_children(stmt, page_store, page_tree_store, Some(&iter), Some(id))?;
            }
            Ok(())
        }

        let page_tree_store = page_store.tree_store();
        self.storage.with_transaction(|tx| {
            tx.execute("DELETE FROM page_tree", &[])?;
            let mut stmt = tx.prepare_cached("
                INSERT INTO page_tree
                (id, parent, position, title, uri, is_pinned)
                VALUES
                (?, ?, ?, ?, ?, ?)
            ")?;
            insert_children(&mut stmt, page_store, &page_tree_store, None, None)?;
            Ok(())
        })
    }
}
