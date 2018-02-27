
pub extern crate rusqlite;

use std::path;
use std::io;
use std::fs;
use std::cell;

const CURRENT_VERSION: u32 = 1;

pub fn do_nothing(_: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> { Ok(()) }

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    InvalidPath(path::PathBuf),
    Io(io::Error),
}

impl From<rusqlite::Error> for Error {

    fn from(error: rusqlite::Error) -> Error { Error::Sqlite(error) }
}

impl From<io::Error> for Error {

    fn from(error: io::Error) -> Error { Error::Io(error) }
}

pub struct Storage {
    conn: cell::RefCell<rusqlite::Connection>,
}

impl Storage {

    pub fn open_in_memory<FI>(init: FI) -> Result<Storage, Error>
    where
        FI: FnOnce(&mut rusqlite::Connection) -> Result<(), rusqlite::Error>,
    {
        let mut conn = rusqlite::Connection::open_in_memory()?;
        common_init(&mut conn)?;
        init(&mut conn)?;
        let conn = cell::RefCell::new(conn);
        Ok(Storage { conn })
    }

    pub fn open<P, FP>(path: P, prepare: FP) -> Result<Storage, Error>
    where
        P: AsRef<path::Path>,
        FP: FnOnce(&mut rusqlite::Connection) -> Result<(), rusqlite::Error>,
    {
        let mut conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?;
        common_prepare(&mut conn)?;
        prepare(&mut conn)?;
        let conn = cell::RefCell::new(conn);
        Ok(Storage { conn })
    }

    pub fn open_or_create<P, FI, FP>(path: P, init: FI, prepare: FP) -> Result<Storage, Error>
    where
        P: AsRef<path::Path>,
        FI: FnOnce(&mut rusqlite::Connection) -> Result<(), rusqlite::Error>,
        FP: FnOnce(&mut rusqlite::Connection) -> Result<(), rusqlite::Error>,
    {
        let path = path.as_ref();

        if path.exists() {
            let mut conn = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            )?;
            common_prepare(&mut conn)?;
            prepare(&mut conn)?;
            let conn = cell::RefCell::new(conn);
            Ok(Storage { conn })
        } else {
            let parent = path.parent().ok_or_else(|| Error::InvalidPath(path.into()))?;
            fs::create_dir_all(parent)?;
            let mut conn = rusqlite::Connection::open(path)?;
            common_init(&mut conn)?;
            init(&mut conn)?;
            let conn = cell::RefCell::new(conn);
            Ok(Storage { conn })
        }
    }

    pub fn with_connection<F, R>(&self, body: F) -> Result<R, Error>
    where F: FnOnce(&rusqlite::Connection) -> Result<R, rusqlite::Error> {
        let conn = self.conn.borrow();
        let result = body(&conn)?;
        Ok(result)
    }

    pub fn with_transaction<F, R>(&self, body: F) -> Result<R, Error>
    where F: FnOnce(&mut rusqlite::Transaction) -> Result<R, rusqlite::Error> {
        let mut conn = self.conn.borrow_mut();
        let mut tx = conn.transaction()?;
        let result = body(&mut tx)?;
        tx.commit()?;
        Ok(result)
    }
}

fn common_init(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("CREATE TABLE app_version (version INTEGER NOT NULL)", &[])?;
    conn.execute("INSERT INTO app_version (version) VALUES (?)", &[&CURRENT_VERSION])?;
    Ok(())
}

fn common_prepare(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("VACUUM", &[])?;
    Ok(())
}

