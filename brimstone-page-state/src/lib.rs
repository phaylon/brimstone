
extern crate brimstone_storage as storage;
extern crate brimstone_domain_settings as domain_settings;

use std::path;

use storage::rusqlite;

fn init_storage(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("
        CREATE TABLE page_host (
            page_id INTEGER NOT NULL UNIQUE,
            host TEXT,
            host_is_domain INTEGER
        )
    ", &[])?;
    conn.execute("
        CREATE TABLE page_request (
            page_id INTEGER NOT NULL,
            host TEXT NOT NULL,
            host_is_domain INTEGER,
            allowed INTEGER
        )
    ", &[])?;
    conn.execute("
        CREATE UNIQUE INDEX idx_page_request
        ON page_request (page_id, host)
    ", &[])?;
    Ok(())
}

pub struct State {
    storage: storage::Storage,
}

impl State {

    pub fn open_in_memory() -> Result<Self, storage::Error> {
        Ok(State {
            storage: storage::Storage::open_in_memory(init_storage)?,
        })
    }

    pub fn open_or_create<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {
        Ok(State {
            storage: storage::Storage::open_or_create(
                path,
                init_storage,
                storage::do_nothing,
            )?,
        })
    }

    pub fn clear(&self) {
        self.storage.with_transaction(|tx| {
            tx.execute("DELETE FROM page_host", &[])?;
            tx.execute("DELETE FROM page_request", &[])?;
            Ok(())
        }).unwrap();
        self.storage.with_connection(|conn| {
            conn.execute("VACUUM", &[])?;
            Ok(())
        }).unwrap();
    }

    pub fn fetch(&self, page_id: u64) -> Option<Data> {
        self.storage.with_connection(|conn| {

            let mut stmt = conn
                .prepare("SELECT host, host_is_domain FROM page_host WHERE page_id = ?")?;
            let mut rows = stmt.query(&[&(page_id as u32)])?;
            let row = match rows.next() {
                Some(row) => row?,
                None => return Ok(None),
            };
            let host: Option<String> = row.get(0);
            let host_is_domain: u32 = row.get(1);
            let host_is_domain = host_is_domain != 0;
            let host = match host {
                Some(host) => domain_settings::Host::new(&host, host_is_domain),
                None => return Ok(None),
            };

            let mut stmt = conn.prepare("
                SELECT host, allowed, host_is_domain
                FROM page_request
                WHERE page_id = ?
            ")?;
            let mut rows = stmt.query(&[&(page_id as u32)])?;
            let mut allowed = Vec::new();
            let mut denied = Vec::new();
            while let Some(row) = rows.next() {
                let row = row?;
                let host: String = row.get(0);
                let is_allowed: u32 = row.get(1);
                let is_allowed = is_allowed != 0;
                let is_domain: u32 = row.get(2);
                let is_domain = is_domain != 0;
                let host = domain_settings::Host::new(&host, is_domain);
                if is_allowed {
                    allowed.push(host);
                } else {
                    denied.push(host);
                }
            }

            Ok(Some(Data { host, allowed, denied }))
        }).unwrap()
    }

    pub fn handle(&self, page_id: u64, host: &domain_settings::Host) -> Handle {
        self.storage.with_transaction(|tx| {
            let current: u32 = tx.query_row(
                "SELECT COUNT(host) FROM page_host WHERE page_id = ? AND host LIKE ?",
                &[&(page_id as u32), &host.as_str()],
                |row| row.get(0),
            )?;
            if current == 0 {
                tx.execute("
                    INSERT OR REPLACE
                    INTO page_host (page_id, host, host_is_domain)
                    VALUES (?, ?, ?)
                ", &[&(page_id as u32), &host.as_str(), &host.is_domain()])?;
                tx.execute(
                    "DELETE FROM page_request WHERE page_id = ?",
                    &[&(page_id as u32)],
                )?;
            }
            Ok(())
        }).unwrap();
        Handle {
            state: self,
            page_id,
        }
    }
}

pub struct Data {
    host: domain_settings::Host,
    allowed: Vec<domain_settings::Host>,
    denied: Vec<domain_settings::Host>,
}

impl Data {

    pub fn host(&self) -> &domain_settings::Host { &self.host }

    pub fn allowed(&self) -> &[domain_settings::Host] { &self.allowed }

    pub fn denied(&self) -> &[domain_settings::Host] { &self.denied }
}

pub struct Handle<'s> {
    state: &'s State,
    page_id: u64,
}

impl<'s> Handle<'s> {

    pub fn push_allowed(&self, host: &domain_settings::Host) {
        self.push(host, true);
    }

    pub fn push_denied(&self, host: &domain_settings::Host) {
        self.push(host, false);
    }

    fn push(&self, host: &domain_settings::Host, allowed: bool) {
        self.state.storage.with_transaction(|tx| {
            tx.execute("
                INSERT OR REPLACE
                INTO page_request (page_id, host, host_is_domain, allowed)
                VALUES (?, ?, ?, ?)
            ", &[
                &(self.page_id as u32),
                &host.as_str(),
                &host.is_domain(),
                &if allowed { 1 } else { 0 },
            ])?;
            Ok(())
        }).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_settings;

    #[test]
    fn storage() {

        let storage = State::open_in_memory().unwrap();
        let handle = storage.handle(23, &domain_settings::Host::new("www.source.com", true));

        handle.push_allowed(&domain_settings::Host::new("www.allowed.com", true));
        handle.push_allowed(&domain_settings::Host::new("www.allowed.com", true));

        handle.push_denied(&domain_settings::Host::new("www1.denied.com", true));
        handle.push_denied(&domain_settings::Host::new("www2.denied.com", true));

        drop(handle);
        let data = storage.fetch(23).expect("data available");
        assert_eq!(data.host(), &domain_settings::Host::new("www.source.com", true));
        assert_eq!(data.allowed(), &[
            domain_settings::Host::new("www.allowed.com", true),
        ]);
        assert_eq!(data.denied(), &[
            domain_settings::Host::new("www1.denied.com", true),
            domain_settings::Host::new("www2.denied.com", true),
        ]);

        storage.clear();
        assert!(storage.fetch(23).is_none());
    }
}
