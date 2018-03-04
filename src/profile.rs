
use std::path;
use std::env;

use xdg;

const XDG_PREFIX: &str = "brimstone";

const FILE_HISTORY: &str = "history.db";
const FILE_DOMAIN_SETTINGS: &str = "domain_settings.db";
const FILE_SESSION: &str = "session.db";
const FILE_SHORTCUTS: &str = "shortcuts.db";
const FILE_BOOKMARKS: &str = "bookmarks.db";

const DIR_PROFILE: &str = "brimstone-profile";
const DIR_CONFIG: &str = "brimstone-config";

#[derive(Debug, Clone)]
pub enum Mode {
    Local,
    Xdg,
    Custom(String),
}

#[derive(Debug)]
pub struct Profile {
    history: path::PathBuf,
    domain_settings: path::PathBuf,
    session: path::PathBuf,
    shortcuts: path::PathBuf,
    bookmarks: path::PathBuf,
}

impl Profile {

    pub fn new(mode: &Mode) -> Profile {
        match *mode {
            Mode::Local => {
                let cwd = env::current_dir().expect("current working directory");
                let dir = cwd.join(DIR_PROFILE);
                let dir_config = dir.join(DIR_CONFIG);
                Profile {
                    history: dir_config.join(FILE_HISTORY),
                    domain_settings: dir_config.join(FILE_DOMAIN_SETTINGS),
                    session: dir_config.join(FILE_SESSION),
                    shortcuts: dir_config.join(FILE_SHORTCUTS),
                    bookmarks: dir_config.join(FILE_BOOKMARKS),
                }
            },
            Mode::Xdg => {
                let base = xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                    .expect("prefixed xdg base directories");
                Profile {
                    history: base.place_config_file(FILE_HISTORY)
                        .expect("history storage file"),
                    domain_settings: base.place_config_file(FILE_DOMAIN_SETTINGS)
                        .expect("domain settings storage file"),
                    session: base.place_config_file(FILE_SESSION)
                        .expect("session storage file"),
                    shortcuts: base.place_config_file(FILE_SHORTCUTS)
                        .expect("shortcuts storage file"),
                    bookmarks: base.place_config_file(FILE_BOOKMARKS)
                        .expect("bookmarks storage file"),
                }
            },
            Mode::Custom(ref root) => {
                let root: path::PathBuf = root.into();
                let dir_config = root.join(DIR_CONFIG);
                Profile {
                    history: dir_config.join(FILE_HISTORY),
                    domain_settings: dir_config.join(FILE_DOMAIN_SETTINGS),
                    session: dir_config.join(FILE_SESSION),
                    shortcuts: dir_config.join(FILE_SHORTCUTS),
                    bookmarks: dir_config.join(FILE_BOOKMARKS),
                }
            },
        }
    }

    pub fn history(&self) -> &path::Path { &self.history }

    pub fn domain_settings(&self) -> &path::Path { &self.domain_settings }

    pub fn session(&self) -> &path::Path { &self.session }

    pub fn shortcuts(&self) -> &path::Path { &self.shortcuts }

    pub fn bookmarks(&self) -> &path::Path { &self.bookmarks }
}
