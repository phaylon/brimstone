
use std::path;
use std::cmp;

extern crate brimstone_storage as storage;

extern crate tendril;
extern crate url;

use storage::rusqlite;

fn init_storage(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute("
        CREATE TABLE third_party_target (
            source_domain TEXT NOT NULL,
            target_domain TEXT NOT NULL
        )
    ", &[])?;
    conn.execute("
        CREATE UNIQUE INDEX idx_third_party_target
        ON third_party_target (source_domain, target_domain)
    ", &[])?;
    Ok(())
}

pub struct Settings {
    storage: storage::Storage,
}

impl Settings {

    pub fn open_in_memory() -> Result<Self, storage::Error> {
        Ok(Settings {
            storage: storage::Storage::open_in_memory(init_storage)?,
        })
    }

    pub fn open<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {
        Ok(Settings {
            storage: storage::Storage::open(path, |_conn| Ok(()))?,
        })
    }

    pub fn open_or_create<P>(path: P) -> Result<Self, storage::Error>
    where P: AsRef<path::Path> {
        Ok(Settings {
            storage: storage::Storage::open_or_create(
                path,
                init_storage,
                storage::do_nothing,
            )?,
        })
    }

    pub fn insert_always_entry(&self, source: &Host) {
        self.storage.with_transaction(|tx| {
            tx.execute("
                INSERT OR IGNORE
                INTO third_party_target (source_domain, target_domain)
                VALUES (?, ?)
            ", &[&source.as_str(), &""])?;
            Ok(())
        }).unwrap();
    }

    pub fn insert_entry(&self, source: &Host, target: &Host) {
        self.storage.with_transaction(|tx| {
            tx.execute("
                INSERT OR IGNORE
                INTO third_party_target (source_domain, target_domain)
                VALUES (?, ?)
            ", &[&source.as_str(), &target.as_str()])?;
            Ok(())
        }).unwrap();
    }

    pub fn remove_always_entry(&self, source: &Host) {
        self.storage.with_transaction(|tx| {
            tx.execute("
                DELETE FROM third_party_target
                WHERE source_domain = ? AND target_domain = ?
            ", &[&source.as_str(), &""])?;
            Ok(())
        }).unwrap();
    }

    pub fn remove_entry(&self, source: &Host, target: &Host) {
        self.storage.with_transaction(|tx| {
            tx.execute("
                DELETE FROM third_party_target
                WHERE source_domain = ? AND target_domain = ?
            ", &[&source.as_str(), &target.as_str()])?;
            Ok(())
        }).unwrap();
    }

    pub fn has_always_entry(&self, source: &Host) -> bool {
        self.storage.with_connection(|conn| {
            let count: u32 = conn.query_row("
                SELECT COUNT(target_domain)
                FROM third_party_target
                WHERE source_domain LIKE ? AND target_domain LIKE ?
            ", &[&source.as_str(), &""], |row| row.get(0))?;
            Ok(count > 0)
        }).unwrap()
    }

    pub fn has_entry(&self, source: &Host, target: &Host) -> bool {
        self.storage.with_connection(|conn| {
            let count: u32 = conn.query_row("
                SELECT COUNT(target_domain)
                FROM third_party_target
                WHERE source_domain LIKE ? AND target_domain LIKE ?
            ", &[&source.as_str(), &target.as_str()], |row| row.get(0))?;
            Ok(count > 0)
        }).unwrap()
    }

    pub fn can_request(&self, source: &Host, target: &Host) -> bool {

        if self.has_always_entry(source) {
            return true;
        }

        let mut current = source.clone();
        while let Some(parent) = current.parent() {
            if self.has_always_entry(&parent) {
                return true;
            }
            current = parent;
        }

        if self.has_entry(source, target) {
            return true;
        }

        let mut current = target.clone();
        while let Some(parent) = current.parent() {
            if self.has_entry(source, &parent) {
                return true;
            }
            current = parent;
        }

        false
    }
}

#[derive(Debug, Clone, Eq, Ord)]
pub struct Domain {
    content: tendril::StrTendril,
    tld: Option<(usize, usize)>,
    main: (usize, usize),
    sub: Vec<(usize, usize)>,
}

impl PartialEq for Domain {

    fn eq(&self, other: &Domain) -> bool {
        self.tld() == other.tld()
            && self.main() == other.main()
            && self.sub() == other.sub()
    }
}

impl PartialOrd for Domain {

    fn partial_cmp(&self, other: &Domain) -> Option<cmp::Ordering> {
        Some(
            self.main().cmp(other.main())
                .then_with(|| match (self.tld(), other.tld()) {
                    (Some(self_tld), Some(other_tld)) => self_tld.cmp(other_tld),
                    (None, None) => cmp::Ordering::Equal,
                    (Some(_), None) => cmp::Ordering::Greater,
                    (None, Some(_)) => cmp::Ordering::Less,
                })
                .then_with(|| {
                    let mut self_sub = self.sub();
                    let mut other_sub = other.sub();
                    self_sub.reverse();
                    other_sub.reverse();
                    let mut index = 0;
                    loop {
                        match (self_sub.get(index), other_sub.get(index)) {
                            (Some(self_item), Some(other_item)) =>
                                match self_item.cmp(other_item) {
                                    cmp::Ordering::Equal => (),
                                    other => return other,
                                },
                            (Some(_), None) => return cmp::Ordering::Greater,
                            (None, Some(_)) => return cmp::Ordering::Less,
                            (None, None) => return cmp::Ordering::Equal,
                        };
                        index += 1;
                    }
                })
        )
    }
}

impl Domain {

    pub fn new(content: &str) -> Domain {
        let mut parts = parse_parts(content);
        assert!(parts.len() > 0);
        let tld =
            if parts.len() > 2 && looks_like_tld(&parts[parts.len() - 2..]) {
                let end = parts.pop().unwrap().1;
                let start = parts.pop().unwrap().0;
                Some((start, end))
            } else if parts.len() > 1 && looks_like_tld(&parts[parts.len() - 1..]) {
                Some(parts.pop().unwrap())
            } else {
                None
            };
        let main = parts.pop().unwrap();
        let sub = parts;
        Domain {
            content: content.to_ascii_lowercase().into(),
            tld,
            main,
            sub,
        }
    }

    fn tld(&self) -> Option<&str> { self.tld.map(|tld| self.resolve(tld)) }
    fn main(&self) -> &str { self.resolve(self.main) }
    fn sub(&self) -> Vec<&str> { self.sub.iter().map(|sub| self.resolve(*sub)).collect() }

    fn resolve(&self, (start, end): (usize, usize)) -> &str {
        &self.content[start..end]
    }

    pub fn parent(&self) -> Option<Domain> {
        if self.sub.len() > 0 {
            Some(Domain {
                content: self.content.clone(),
                tld: self.tld,
                main: self.main,
                sub: self.sub[1..].into(),
            })
        } else {
            None
        }
    }

    pub fn is_parent_of(&self, other: &Domain) -> bool {
        self.tld() == other.tld()
            && self.main() == other.main()
            && {
                let mut self_sub = self.sub();
                let mut other_sub = other.sub();
                self_sub.reverse();
                other_sub.reverse();
                other_sub.len() > self_sub.len()
                    && &other_sub[..self_sub.len()] == &self_sub[..]
            }
    }

    pub fn is_related_to(&self, other: &Domain) -> bool {
        self == other
            || self.is_parent_of(other)
            || other.is_parent_of(self)
    }

    pub fn as_str(&self) -> &str {
        let start = self.sub.get(0)
            .map(|pos| pos.0)
            .unwrap_or_else(|| self.main.0);
        &self.content[start..]
    }
}

#[derive(Debug, Clone, Eq, Ord)]
pub enum Host {
    Ip(tendril::StrTendril),
    Domain(Domain),
}

impl PartialOrd for Host {

    fn partial_cmp(&self, other: &Host) -> Option<cmp::Ordering> {
        Some(match (self, other) {
            (&Host::Ip(ref self_ip), &Host::Ip(ref other_ip)) =>
                self_ip.cmp(other_ip),
            (&Host::Domain(ref self_domain), &Host::Domain(ref other_domain)) =>
                self_domain.cmp(other_domain),
            (&Host::Ip(_), &Host::Domain(_)) =>
                cmp::Ordering::Less,
            (&Host::Domain(_), &Host::Ip(_)) =>
                cmp::Ordering::Greater,
        })
    }
}

impl PartialEq for Host {

    fn eq(&self, other: &Host) -> bool {
        match (self, other) {
            (&Host::Ip(ref ip_self), &Host::Ip(ref ip_other)) =>
                ip_self == ip_other,
            (&Host::Domain(ref domain_self), &Host::Domain(ref domain_other)) =>
                domain_self == domain_other,
            _ => false,
        }
    }
}

impl Host {

    pub fn new(value: &str, is_domain: bool) -> Host {
        if is_domain {
            Host::Domain(Domain::new(value))
        } else {
            Host::Ip(value.into())
        }
    }
    
    pub fn from_uri(uri: &url::Url) -> Option<Host> {
        uri.host().and_then(|host| match host {
            url::Host::Domain(domain) => Some(Host::Domain(Domain::new(&domain))),
            _ => uri.host_str().map(|host| Host::Ip(host.into())),
        })
    }

    pub fn parent(&self) -> Option<Host> {
        match *self {
            Host::Ip(_) => None,
            Host::Domain(ref domain) => domain.parent().map(|parent| Host::Domain(parent)),
        }
    }

    pub fn is_domain(&self) -> bool {
        match *self {
            Host::Domain(_) => true,
            _ => false,
        }
    }

    pub fn is_related_to(&self, other: &Host) -> bool {
        self == other
        || self.is_parent_of(other)
        || other.is_parent_of(self)
    }

    pub fn is_parent_of(&self, other: &Host) -> bool {
        match (self, other) {
            (&Host::Domain(ref self_domain), &Host::Domain(ref other_domain)) =>
                self_domain.is_parent_of(other_domain),
            _ => false,
        }
    }

    pub fn to_expanded(&self) -> Vec<Host> {
        match *self {
            Host::Ip(_) => vec![self.clone()],
            Host::Domain(ref domain) => {
                let mut done = vec![self.clone()];
                let mut current = domain.clone();
                while let Some(parent) = current.parent() {
                    done.push(Host::Domain(parent.clone()));
                    current = parent;
                }
                done
            },
        }
    }

    pub fn as_str(&self) -> &str {
        match *self {
            Host::Ip(ref ip) => ip,
            Host::Domain(ref domain) => domain.as_str(),
        }
    }
}

fn looks_like_tld(parts: &[(usize, usize)]) -> bool {
    if parts.len() == 0 {
        true
    } else if parts.len() == 1 {
        parts[0].1 - parts[0].0 <= 3
    } else if parts.len() == 2 {
        parts[0].1 - parts[0].0 <= 2
        &&
        parts[1].1 - parts[1].0 <= 2
    } else {
        false
    }
}

fn parse_parts(domain: &str) -> Vec<(usize, usize)> {
    let mut parts = Vec::new();
    let mut offset = 0;
    while let Some(len) = domain[offset..].find('.') {
        parts.push((offset, offset+len));
        offset = offset + len + 1;
    }
    parts.push((offset, domain.len()));
    parts
}

#[cfg(test)]
mod tests {
    use url;
    use super::*;

    fn make_uri(uri: &str) -> url::Url {
        url::Url::parse(uri).unwrap()
    }

    fn make_host(uri: &str) -> Host {
        Host::from_uri(&make_uri(uri)).unwrap()
    }

    fn make_domain(host: &str) -> Host {
        Host::new(host, true)
    }

    #[test]
    fn storage() {
        let domains = Settings::open_in_memory().unwrap();

        let always = make_domain("www.always.com");
        domains.insert_always_entry(&always);
        assert!(domains.has_always_entry(&always));
        assert!(domains.can_request(&always, &always));
        assert!(domains.can_request(&always, &make_domain("www2.always.com")));
        assert!(domains.can_request(&always, &make_domain("www.other.co.uk")));

        let some_source = make_domain("source.com");
        let some_target = make_domain("target.com");
        domains.insert_entry(&some_source, &some_target);
        assert!(domains.has_entry(&some_source, &some_target));
        assert!(!domains.has_always_entry(&some_source));
        assert!(domains.can_request(&some_source, &some_target));
        assert!(domains.can_request(&some_source, &make_domain("www.target.com")));
        assert!(domains.can_request(&some_source, &make_domain("foo.www.target.com")));
        assert!(!domains.can_request(&some_source, &always));
    }

    #[test]
    fn domain_eq() {
        let a = Domain::new("www.example.com");
        let b = Domain::new("WWW.EXAMPLE.COM");
        assert_eq!(a, b);
    }

    #[test]
    fn domain_cmp() {
        let mut domains = vec![
            Domain::new("example.com"),
            Domain::new("localhost.com"),
            Domain::new("www2.example.com"),
            Domain::new("localhost"),
            Domain::new("www.example.com"),
            Domain::new("baz.foo.co.uk"),
            Domain::new("bar.foo.co.uk"),
        ];
        domains.sort();
        assert_eq!(&domains, &[
            Domain::new("example.com"),
            Domain::new("www.example.com"),
            Domain::new("www2.example.com"),
            Domain::new("bar.foo.co.uk"),
            Domain::new("baz.foo.co.uk"),
            Domain::new("localhost"),
            Domain::new("localhost.com"),
        ]);
    }

    #[test]
    fn expanded() {

        let host = make_host("http://www.example.com");
        let expanded = host.to_expanded();
        assert_eq!(&expanded, &[
            Host::new("www.example.com", true),
            Host::new("example.com", true),
        ]);

        let host = make_host("http://www.example.co.uk");
        let expanded = host.to_expanded();
        assert_eq!(&expanded, &[
            Host::new("www.example.co.uk", true),
            Host::new("example.co.uk", true),
        ]);

        let host = make_host("http://www.localhost");
        let expanded = host.to_expanded();
        assert_eq!(&expanded, &[
            Host::new("www.localhost", true),
            Host::new("localhost", true),
        ]);
    }

    #[test]
    fn host_parse() {
        let host = make_host("https://www.example.com");
        match host {
            Host::Domain(ref domain) => {
                assert_eq!(domain.tld(), Some("com"));
                assert_eq!(domain.main(), "example");
                assert_eq!(domain.sub(), &["www"]);
            },
            other => panic!("wrong host: {:?}", other),
        }
        let host = make_host("http://127.0.0.1");
        match host {
            Host::Ip(ref full) => {
                assert_eq!(full.as_ref(), "127.0.0.1");
            },
            other => panic!("wrong host: {:?}", other),
        }
    }

    #[test]
    fn host_eq() {
        let host_a = make_host("http://www.example.com");
        let host_b = make_host("https://WWW.Example.Com");
        assert_eq!(host_a, host_b);
    }

    #[test]
    fn parent() {
        let parent = make_host("http://example.com");
        let child = make_host("http://www.example.com");
        assert!(parent.is_parent_of(&child));
        assert!(!child.is_parent_of(&parent));
    }

    #[test]
    fn related() {
        let parent = make_host("http://example.com");
        let child = make_host("http://www.example.com");
        assert!(parent.is_related_to(&child));
        assert!(child.is_related_to(&parent));
        assert!(parent.is_related_to(&parent));
    }

    #[test]
    fn parts() {
        assert_eq!(
            &parse_parts("www.example.com"),
            &[(0, 3), (4, 11), (12, 15)],
        );
        assert_eq!(
            &parse_parts("localhost"),
            &[(0, 9)],
        );
    }
}
