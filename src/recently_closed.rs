
use std::cell;

use page_store;
use signal;
use text;

#[derive(Debug)]
pub struct Page {
    pub id: page_store::Id,
    pub position: Vec<(Option<page_store::Id>, u32)>,
    pub title: Option<text::RcString>,
    pub uri: text::RcString,
}

pub struct State {
    items: cell::RefCell<Vec<Page>>,
    change_notifier: signal::Notifier<State, ()>,
}

impl State {

    pub fn new() -> State {
        State {
            items: cell::RefCell::new(Vec::new()),
            change_notifier: signal::Notifier::new(),
        }
    }

    pub fn get_count(&self) -> usize { self.items.borrow().len() }

    pub fn is_empty(&self) -> bool { self.get_count() == 0 }

    fn find_index(&self, id: page_store::Id) -> Option<usize> {
        let pages = self.items.borrow();
        for index in 0..pages.len() {
            if pages[index].id == id {
                return Some(index);
            }
        }
        None
    }

    pub fn pull(&self, id: page_store::Id) -> Option<Page> {
        log_debug!("pull page {}", id);
        let index = self.find_index(id)?;
        let page = Some(self.items.borrow_mut().remove(index));
        self.change_notifier.emit(self, &());
        page
    }

    pub fn pull_most_recent(&self) -> Option<Page> {
        log_debug!("pull most recent");
        let page = self.items.borrow_mut().pop();
        if page.is_some() {
            self.change_notifier.emit(self, &());
        }
        page
    }

    pub fn push(&self, page: Page) {
        use dynamic::{ BorrowMutIn };

        log_debug!("push page {}", page.id);
        self.items.borrow_mut_in(|mut items| {
            items.push(page);
            if items.len() > 10 {
                items.pop();
            }
        });
        self.change_notifier.emit(self, &())
    }

    pub fn iterate_pages<F>(&self, mut callback: F) where F: FnMut(&Page) {
        let items = self.items.borrow();
        for item in items.iter() {
            callback(item);
        }
    }

    pub fn on_change<F>(&self, callback: F) where F: Fn(&Self, &()) + 'static {
        self.change_notifier.connect(callback);
    }
}
