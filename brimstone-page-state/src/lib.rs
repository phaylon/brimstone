
extern crate nix;
extern crate dbus;
extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate brimstone_domain_settings as domain_settings;

use std::thread;
use std::rc;
use std::cell;
use std::collections;
use std::sync;
use std::path;

const PATH_OBJECT: &str = "/at/dunkelheit/brimstone/page_state";
const INTERFACE: &str = "at.dunkelheit.brimstone.page_state";
const METHOD_SET_PAGE_HOST: &str = "SetPageHost";
const METHOD_QUIT: &str = "Quit";

#[derive(Debug, Deserialize, Serialize)]
pub struct InitArguments {
    pub instance: String,
    pub domain_settings_path: path::PathBuf,
}

fn unpack_host(host: &domain_settings::Host) -> (String, bool) {
    (host.as_str().into(), host.is_domain())
}

fn pack_host(&(ref host, is_domain): &(String, bool)) -> domain_settings::Host {
    domain_settings::Host::new(host, is_domain)
}

pub struct Store {
    pages: collections::HashMap<u64, StoreEntry>,
}

impl Store {

    pub fn get_data(&self, page_id: u64) -> Option<Data> {
        self.pages.get(&page_id).map(|data| Data {
            host: pack_host(&data.host),
            allowed: data.allowed.iter().map(|host| pack_host(host)).collect(),
            denied: data.denied.iter().map(|host| pack_host(host)).collect(),
        })
    }

    fn push(
        &mut self,
        page_id: u64,
        source: &domain_settings::Host,
        target: &domain_settings::Host,
        is_allowed: bool,
    ) {
        let entry = self.pages.entry(page_id).or_insert_with(|| StoreEntry {
            host: unpack_host(source),
            allowed: Vec::new(),
            denied: Vec::new(),
        });
        if &pack_host(&entry.host) != source {
            entry.allowed.clear();
            entry.denied.clear();
        }
        if is_allowed {
            entry.allowed.push(unpack_host(target));
        } else {
            entry.denied.push(unpack_host(target));
        }
    }
}

pub struct StoreEntry {
    host: (String, bool),
    allowed: Vec<(String, bool)>,
    denied: Vec<(String, bool)>,
}

#[derive(Debug)]
pub struct Server {
    name: String,
}

impl Server {

    pub fn name(&self) -> &str { &self.name }
}

impl Drop for Server {

    fn drop(&mut self) {
        println!("DROP");
        let client = Client::new(&self.name);
        client.quit();
    }
}

pub fn run_server() -> (Server, sync::Arc<sync::Mutex<Store>>) {
    let name = format!("{}.instance-{}", INTERFACE, nix::unistd::getpid());
    let store = sync::Arc::new(sync::Mutex::new(Store {
        pages: collections::HashMap::new(),
    }));
    thread::spawn({
        let name = name.clone();
        let store = store.clone();
        move || {
            let quit_flag = rc::Rc::new(cell::Cell::new(false));
            let conn = dbus::Connection::get_private(dbus::BusType::Session)
                .expect("dbus session bus access");
            conn.register_name(&name, dbus::NameFlag::ReplaceExisting as u32)
                .expect(&format!("dbus name {:?}", name));
            let fac = dbus::tree::Factory::new_fn::<()>();
            let tree = fac
                .tree(())
                .add(fac.object_path(PATH_OBJECT, ()).add(
                    fac.interface(INTERFACE, ())
                        .add_m({
                            let quit_flag = quit_flag.clone();
                            fac.method(METHOD_QUIT, (), move |_m| {
                                quit_flag.set(true);
                                Ok(Vec::new())
                            })
                        })
                        .add_m(
                            fac.method(METHOD_SET_PAGE_HOST, (), move |m| {
                                let mut args = m.msg.iter_init();
                                let source = {
                                    let host: &str = args.read()?;
                                    let is_domain: bool = args.read()?;
                                    domain_settings::Host::new(host, is_domain)
                                };
                                let target = {
                                    let host: &str = args.read()?;
                                    let is_domain: bool = args.read()?;
                                    domain_settings::Host::new(host, is_domain)
                                };
                                let is_allowed: bool = args.read()?;
                                let page_id: u64 = args.read()?;
                                store.lock()
                                    .expect("page state storage access")
                                    .push(page_id, &source, &target, is_allowed);
                                Ok(Vec::new())
                            })
                            .inarg::<&str, _>("source_host")
                            .inarg::<bool, _>("source_host_is_domain")
                            .inarg::<&str, _>("target_host")
                            .inarg::<bool, _>("target_host_is_domain")
                            .inarg::<bool, _>("is_allowed")
                            .inarg::<u64, _>("page_id")
                        )
                ));
            tree.set_registered(&conn, true)
                .expect("dbus tree registered");
            conn.add_handler(tree);
            loop {
                if quit_flag.get() {
                    return;
                }
                conn.incoming(1000).next();
            }
        }
    });
    (Server { name }, store)
}

pub struct Client {
    name: String,
    conn: dbus::Connection,
}

impl Client {

    pub fn new(name: &str) -> Client {
        Client {
            name: name.into(),
            conn: dbus::Connection::get_private(dbus::BusType::Session)
                .expect("dbus client connection"),
        }
    }

    pub fn quit(&self) {
        let call = dbus::Message::new_method_call(
            &self.name,
            PATH_OBJECT,
            INTERFACE,
            METHOD_QUIT,
        ).expect("dbus quit method message construction");
        self.conn.send(call).expect("dbus quit method call dispatch");
    }

    pub fn push(
        &self,
        page_id: u64,
        source: &domain_settings::Host,
        target: &domain_settings::Host,
        is_allowed: bool,
    ) {
        let call =
            dbus::Message::new_method_call(
                &self.name,
                PATH_OBJECT,
                INTERFACE,
                METHOD_SET_PAGE_HOST,
            ).expect("dbus push method message construction")
            .append1(source.as_str())
            .append1(source.is_domain())
            .append1(target.as_str())
            .append1(target.is_domain())
            .append1(is_allowed)
            .append1(page_id);
        self.conn.send(call).expect("dbus push method call dispatch");
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

