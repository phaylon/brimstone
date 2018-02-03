
use std::fs;
use std::io;
use std::path;
use std::thread;
use std::sync;
use std::cmp;
use std::cell;
use std::rc;

use rusqlite;
use gtk;

use page_store;

pub struct Storage {
    conn: rusqlite::Connection,
}

#[derive(Debug)]
pub enum OpenStorageError {
    File,
    Connection(rusqlite::Error),
    Prepare(rusqlite::Error),
}

#[derive(Debug)]
pub enum CreateStorageError {
    Directory(io::Error),
    Connection(rusqlite::Error),
    Init(rusqlite::Error),
    InvalidPath(path::PathBuf),
}

#[derive(Debug)]
pub enum OpenOrCreateStorageError {
    Open(OpenStorageError),
    Create(CreateStorageError),
}

impl From<CreateStorageError> for OpenOrCreateStorageError {

    fn from(error: CreateStorageError) -> Self { OpenOrCreateStorageError::Create(error) }
}

impl From<OpenStorageError> for OpenOrCreateStorageError {

    fn from(error: OpenStorageError) -> Self { OpenOrCreateStorageError::Open(error) }
}

fn init_storage(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    log_debug!("initializing storage");
    conn.execute("
        CREATE TABLE page_info (
            id INTEGER PRIMARY KEY,
            title TEXT,
            uri TEXT,
            is_pinned INTEGER
        )
    ", &[])?;
    conn.execute("
        CREATE TABLE page_tree (
            id INTEGER PRIMARY KEY,
            parent INTEGER,
            position INTEGER NOT NULL
        )
    ", &[])?;
    Ok(())
}

type InfoRows = Vec<InfoRow>;
type TreeRows = Vec<TreeRow>;

struct InfoRow {
    id: page_store::Id,
    title: Option<String>,
    uri: String,
    is_pinned: bool,
}

struct TreeRow {
    id: page_store::Id,
    parent: Option<page_store::Id>,
    position: u32,
}

pub type RcNode = rc::Rc<cell::RefCell<Node>>;
pub type Nodes = Vec<RcNode>;

type ParentMap = Vec<(Option<page_store::Id>, RcNode)>;

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub title: Option<String>,
    pub uri: String,
    pub is_pinned: bool,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: page_store::Id,
    pub position: u32,
    pub children: Nodes,
    pub info: Option<NodeInfo>,
}

struct IdSeq {
    current: page_store::Id,
}

impl IdSeq {

    fn new() -> IdSeq {
        IdSeq {
            current: 1,
        }
    }

    fn next(&mut self) -> page_store::Id {
        let current = self.current;
        self.current = current.checked_add(1).unwrap();
        current
    }
}

fn get_info_rows(conn: &rusqlite::Connection) -> Result<InfoRows, rusqlite::Error> {
    log_trace!("loading page info data");
    let mut stmt = conn.prepare("SELECT id, title, uri, is_pinned FROM page_info")?;
    let mut rows = stmt.query(&[])?;
    let mut info = Vec::new();
    while let Some(row) = rows.next() {
        let row = row?;
        info.push(InfoRow {
            id: row.get(0),
            title: row.get(1),
            uri: row.get(2),
            is_pinned: row.get::<_, Option<bool>>(3).unwrap_or(false),
        });
    }
    log_trace!("found {} page info row(s)", info.len());
    Ok(info)
}

fn get_tree_rows(conn: &rusqlite::Connection) -> Result<TreeRows, rusqlite::Error> {
    log_trace!("loading page tree data");
    let mut stmt = conn.prepare("SELECT id, parent, position FROM page_tree")?;
    let mut rows = stmt.query(&[])?;
    let mut tree = Vec::new();
    while let Some(row) = rows.next() {
        let row = row?;
        tree.push(TreeRow {
            id: row.get(0),
            parent: row.get(1),
            position: row.get(2),
        });
    }
    log_trace!("found {} page tree row(s)", tree.len());
    Ok(tree)
}

fn extract<T, F>(items: &mut Vec<T>, pred: F) -> Option<T>
where F: Fn(&T) -> bool {
    for index in 0..items.len() {
        if pred(&items[index]) {
            return Some(items.remove(index));
        }
    }
    None
}

fn insert_nodes(
    tx: &rusqlite::Transaction,
    seq: &mut IdSeq,
    parent: Option<page_store::Id>,
    nodes: &Nodes,
) -> Result<(), rusqlite::Error> {

    let no_string: Option<String> = None;
    let empty_string = String::new();

    for index in 0..nodes.len() {
        let node = nodes[index].borrow();
        let id = seq.next();
        tx.execute(
            "INSERT INTO page_tree (id, parent, position) VALUES (?, ?, ?)",
            &[&id, &parent, &(index as u32)],
        )?;
        let insert_sql = "INSERT INTO page_info (id, title, uri, is_pinned) VALUES (?, ?, ?, ?)";
        match node.info {
            Some(ref info) => tx.execute(insert_sql, &[
                &id,
                &info.title,
                &info.uri,
                &info.is_pinned,
            ])?,
            None => tx.execute(insert_sql, &[
                &id,
                &no_string,
                &empty_string,
                &0,
            ])?,
        };
        insert_nodes(tx, seq, Some(id), &node.children)?;
    }
    Ok(())
}

fn convert_tree_rows_to_nodes(tree: TreeRows) -> ParentMap {
    tree.into_iter().map(|row| (
        row.parent,
        rc::Rc::new(cell::RefCell::new(Node {
            id: row.id,
            position: row.position,
            children: Vec::new(),
            info: None,
        }))
    )).collect()
}

fn find_pinned_ids(info: &InfoRows) -> Vec<page_store::Id> {
    let mut pinned = Vec::new();
    for row in info {
        if row.is_pinned {
            pinned.push(row.id);
        }
    }
    log_trace!("found pinned: {:?}", &pinned);
    pinned
}

fn inflate_tree(nodes: &ParentMap, pinned: &[page_store::Id]) -> Nodes {
    log_trace!("inflating tree from {} node(s)", nodes.len());
    let mut roots = Vec::new();
    'nodes: for &(parent, ref node) in nodes {
        if let Some(parent) = parent {
            if !pinned.contains(&node.borrow().id) {
                for &(_, ref potential_parent) in nodes {
                    if parent == potential_parent.borrow().id {
                        potential_parent.borrow_mut().children.push(node.clone());
                        continue 'nodes;
                    }
                }
            }
            roots.push(node.clone());
        } else {
            roots.push(node.clone());
        }
    }
    log_trace!("found {} root node(s)", roots.len());
    roots
}

fn attach_info_rows_to_nodes(nodes: &ParentMap, info: &mut InfoRows) {
    log_trace!("attaching info to nodes");
    for &(_, ref node) in nodes {
        let id = node.borrow().id;
        let row = match extract(info, |row| row.id == id) {
            Some(row) => row,
            None => continue,
        };
        node.borrow_mut().info = Some(NodeInfo {
            title: row.title,
            uri: row.uri,
            is_pinned: row.is_pinned,
        });
    }
}

fn sort_node_children(nodes: &ParentMap) {
    log_trace!("sorting node children");
    for &(_, ref node) in nodes {
        node.borrow_mut().children.sort_by_key(|child| child.borrow().position);
    }
}

fn sort_root_nodes(roots: &mut Nodes) {
    log_trace!("sorting root nodes");
    roots.sort_by_key(|child| (
        if let Some(ref info) = child.borrow().info {
            if info.is_pinned { 0 } else { 1 }
        } else {
            1
        },
        child.borrow().position,
    ));
}

fn repopulate(conn: &mut rusqlite::Connection, roots: &Nodes) -> Result<(), rusqlite::Error> {
    log_trace!("repopulating corrected session");
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM page_tree", &[])?;
    tx.execute("DELETE FROM page_info", &[])?;
    let mut seq = IdSeq::new();
    insert_nodes(&tx, &mut seq, None, &roots)?;
    Ok(())
}

fn vacuum(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    log_trace!("vacuum");
    conn.execute("VACUUM", &[])?;
    Ok(())
}

fn load_nodes_to_tree(conn: &mut rusqlite::Connection) -> rusqlite::Result<Nodes> {
    log_trace!("constructing tree");
    
    let mut info = get_info_rows(conn)?;
    let tree = get_tree_rows(conn)?;

    let nodes = convert_tree_rows_to_nodes(tree);
    let pinned = find_pinned_ids(&info);
    let mut roots = inflate_tree(&nodes, &pinned);

    attach_info_rows_to_nodes(&nodes, &mut info);
    sort_node_children(&nodes);
    sort_root_nodes(&mut roots);

    log_trace!("tree construction complete");
    Ok(roots)
}

fn prepare_storage(conn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    log_trace!("preparing storage for use");

    let roots = load_nodes_to_tree(conn)?;

    repopulate(conn, &roots)?;
    vacuum(conn)?;

    log_trace!("storage preparation complete");
    Ok(())
}

impl Storage {

    pub fn open_or_create<P>(path: P) -> Result<Storage, OpenOrCreateStorageError>
    where P: AsRef<path::Path> {
        let path = path.as_ref();
        log_trace!("storage path {:?}", path);
        if path.exists() {
            log_trace!("storage exists");
            if !path.is_file() {
                return Err(OpenStorageError::File.into());
            }
            let mut conn = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            ).map_err(OpenStorageError::Connection)?;
            prepare_storage(&mut conn)
                .map_err(OpenStorageError::Prepare)?;
            Ok(Storage { conn })
        } else {
            log_trace!("storage does not exist");
            let parent = path
                .parent()
                .ok_or_else(|| CreateStorageError::InvalidPath(path.into()))?;
            fs::create_dir_all(parent)
                .map_err(CreateStorageError::Directory)?;
            let conn = rusqlite::Connection::open(path)
                .map_err(CreateStorageError::Connection)?;
            init_storage(&conn)
                .map_err(CreateStorageError::Init)?;
            Ok(Storage { conn })
        }
    }

    pub fn load_tree(&mut self) -> Result<Nodes, rusqlite::Error> {
        load_nodes_to_tree(&mut self.conn)
    }

    pub fn find_pinned_ids(&mut self) -> Vec<page_store::Id> {
        let mut pinned = Vec::new();
        let mut stmt = self.conn.prepare("SELECT id FROM page_info WHERE is_pinned = 1").unwrap();
        let mut rows = stmt.query(&[]).unwrap();
        while let Some(row) = rows.next() {
            let row = row.unwrap();
            pinned.push(row.get(0));
        }
        pinned
    }

    pub fn find_highest_id(&mut self) -> page_store::Id {

        let page_tree_count: u32 = self.conn
            .query_row("SELECT COUNT(id) FROM page_tree", &[], |row| row.get(0)).unwrap();
        let page_tree_id =
            if page_tree_count > 0 {
                self.conn
                    .query_row("SELECT MAX(id) FROM page_tree", &[], |row| row.get(0))
                    .unwrap()
            } else {
                0
            };

        let page_info_count: u32 = self.conn
            .query_row("SELECT COUNT(id) FROM page_info", &[], |row| row.get(0)).unwrap();
        let page_info_id =
            if page_info_count > 0 {
                self.conn
                    .query_row("SELECT MAX(id) FROM page_info", &[], |row| row.get(0))
                    .unwrap()
            } else {
                0
            };

        let highest = cmp::max(page_info_id, page_tree_id);
        log_trace!("highest page id found is {}", highest);
        highest
    }

    fn modify(&mut self) -> Result<Modify, rusqlite::Error> {
        let Storage { ref mut conn } = *self;
        Ok(Modify {
            transaction: conn.transaction()?,
        })
    }
}

struct Modify<'conn> {
    transaction: rusqlite::Transaction<'conn>,
}

impl<'conn> Modify<'conn> {

    fn update_title(&self, id: page_store::Id, title: String) -> Result<(), UpdateError> {
        self.transaction.execute(
            "UPDATE page_info SET title = ? WHERE id = ?",
            &[&title, &id],
        )?;
        Ok(())
    }

    fn update_uri(&self, id: page_store::Id, uri: String) -> Result<(), UpdateError> {
        self.transaction.execute(
            "UPDATE page_info SET uri = ? WHERE id = ?",
            &[&uri, &id],
        )?;
        Ok(())
    }

    fn update_is_pinned(&self, id: page_store::Id, is_pinned: bool) -> Result<(), UpdateError> {
        self.transaction.execute(
            "UPDATE page_info SET is_pinned = ? WHERE id = ?",
            &[&is_pinned, &id],
        )?;
        Ok(())
    }

    fn update_create(
        &mut self,
        id: page_store::Id,
        uri: String,
        parent: Option<page_store::Id>,
        position: u32,
    ) -> Result<(), UpdateError> {
        self.transaction.execute(
            "INSERT INTO page_info (id, uri) VALUES (?, ?)",
            &[&id, &uri],
        )?;
        self.transaction.execute(
            "UPDATE page_tree SET position = position + 1 WHERE parent = ? AND position >= ?",
            &[&parent, &position],
        )?;
        self.transaction.execute(
            "INSERT INTO page_tree (id, parent, position) VALUES (?, ?, ?)",
            &[&id, &parent, &position],
        )?;
        Ok(())
    }

    fn update_remove(&mut self, id: page_store::Id) -> Result<(), UpdateError> {
        let position: u32 = self.transaction.query_row(
            "SELECT position FROM page_tree WHERE id = ?",
            &[&id],
            |row| row.get(0),
        )?;
        let parent: Option<page_store::Id> = self.transaction.query_row(
            "SELECT parent FROM page_tree WHERE id = ?",
            &[&id],
            |row| row.get(0),
        )?;
        self.transaction.execute("DELETE FROM page_info WHERE id = ?", &[&id])?;
        self.transaction.execute("DELETE FROM page_tree WHERE id = ?", &[&id])?;
        self.transaction.execute(
            "UPDATE page_tree SET position = position - 1 WHERE parent = ? AND position > ?",
            &[&parent, &position],
        )?;
        Ok(())
    }

    fn update_tree(&mut self, tree: Tree) -> Result<(), UpdateError> {
        self.transaction.execute("DELETE FROM page_tree", &[])?;
        let mut stmt = self.transaction.prepare(
            "INSERT INTO page_tree (id, parent, position) VALUES (?, ?, ?)",
        )?;
        for &(id, parent, position) in &tree.nodes {
            stmt.execute(&[&id, &parent, &position])?;
        }
        Ok(())
    }

    fn update(&mut self, update: Update) -> Result<(), UpdateError> {
        match update {
            Update::Title { id, title } =>
                self.update_title(id, title),
            Update::Uri { id, uri } =>
                self.update_uri(id, uri),
            Update::Create { id, uri, parent, position } =>
                self.update_create(id, uri, parent, position),
            Update::Remove { id } =>
                self.update_remove(id),
            Update::Tree { tree } =>
                self.update_tree(tree),
            Update::IsPinned { id, is_pinned } =>
                self.update_is_pinned(id, is_pinned),
        }
    }

    fn done(self) -> Result<(), UpdateError> {
        self.transaction.commit()?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum UpdateError {
    Store(rusqlite::Error),
    PageId(page_store::Id),
}

impl From<rusqlite::Error> for UpdateError {

    fn from(error: rusqlite::Error) -> Self { UpdateError::Store(error) }
}

enum Update {
    Title { id: page_store::Id, title: String },
    Uri { id: page_store::Id, uri: String },
    IsPinned { id: page_store::Id, is_pinned: bool },
    Create { id: page_store::Id, uri: String, parent: Option<page_store::Id>, position: u32 },
    Remove { id: page_store::Id },
    Tree { tree: Tree },
}

pub struct Updater {
    handle: Option<thread::JoinHandle<()>>,
    sender: sync::mpsc::Sender<Update>,
}

impl Updater {

    pub fn new(mut storage: Storage) -> Updater {
        let (sender, receiver) = sync::mpsc::channel();
        log_debug!("lauching updater thread");
        let handle = Some(thread::spawn(move || {
            loop {
                let update = match receiver.recv() {
                    Ok(update) => update,
                    Err(_) => return,
                };
                let mut modify = storage.modify().unwrap();
                modify.update(update).unwrap();
                'updates: loop {
                    let update = match receiver.try_recv() {
                        Ok(update) => update,
                        Err(sync::mpsc::TryRecvError::Empty) => break 'updates,
                        Err(_) => return,
                    };
                    modify.update(update).unwrap();
                }
                modify.done().unwrap();
            }
        }));
        Updater { sender, handle }
    }

    pub fn update_title(&self, id: page_store::Id, title: String) {
        log_debug!("title update for {} to {:?}", id, &title);
        self.sender.send(Update::Title { id, title }).unwrap();
    }

    pub fn update_uri(&self, id: page_store::Id, uri: String) {
        log_debug!("uri update for {} to {:?}", id, &uri);
        self.sender.send(Update::Uri { id, uri }).unwrap();
    }

    pub fn update_is_pinned(&self, id: page_store::Id, is_pinned: bool) {
        log_debug!("pin state update for {} to {:?}", id, is_pinned);
        self.sender.send(Update::IsPinned { id, is_pinned }).unwrap();
    }

    pub fn update_create(
        &self,
        id: page_store::Id,
        uri: String,
        parent: Option<page_store::Id>,
        position: u32,
    ) {
        log_debug!("creation update for {}", id);
        self.sender.send(Update::Create { id, uri, parent, position }).unwrap();
    }

    pub fn update_remove(&self, id: page_store::Id) {
        log_debug!("removal update for {}", id);
        self.sender.send(Update::Remove { id }).unwrap();
    }

    pub fn update_tree(&self, page_tree_store: &gtk::TreeStore) {
        log_debug!("tree structure update");
        self.sender.send(Update::Tree {
            tree: Tree::from_page_tree_store(page_tree_store),
        }).unwrap();
    }
}

impl Drop for Updater {

    fn drop(&mut self) {
        log_debug!("waiting for updater thread to finish");
        self.handle.take().map(|handle| handle.join().unwrap());
    }
}

type TreeNodes = Vec<(page_store::Id, Option<page_store::Id>, u32)>;

struct Tree {
    nodes: TreeNodes,
}

impl Tree {

    fn from_page_tree_store(page_tree_store: &gtk::TreeStore) -> Tree {
        use gtk::{ TreeModelExt, Cast };
        use page_tree_store;

        log_trace!("collecting tree structure");

        fn populate(
            model: &gtk::TreeModel,
            nodes: &mut TreeNodes,
            parent: Option<&gtk::TreeIter>,
            parent_id: Option<page_store::Id>,
        ) {
            for index in 0..model.iter_n_children(parent) {
                let iter = model.iter_nth_child(parent, index).unwrap();
                let id = page_tree_store::get::id(&model, &iter);
                nodes.push((id, parent_id, index as u32));
                populate(model, nodes, Some(&iter), Some(id));
            }
        }

        let mut nodes = Vec::new();
        populate(&page_tree_store.clone().upcast(), &mut nodes, None, None);
        
        log_trace!("tree structure collected");

        Tree { nodes }
    }
}
