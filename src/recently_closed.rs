
use std::cell;

use page_store;

#[derive(Debug)]
pub struct Page {
    pub id: page_store::Id,
    pub position: Vec<(Option<page_store::Id>, u32)>,
    pub title: Option<String>,
    pub uri: String,
}

pub struct State {
    items: cell::RefCell<Vec<Page>>,
}

impl State {

    pub fn new() -> State {
        State {
            items: cell::RefCell::new(Vec::new()),
        }
    }

    pub fn push(&self, page: Page) {
        log_debug!("pushed {:?}", &page);
        let mut items = self.items.borrow_mut();
        items.push(page);
        if items.len() > 10 {
            items.pop();
        }
    }
}
