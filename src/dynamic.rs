
use std::cell;

pub trait BorrowIn {

    type Item;

    fn borrow_in<'r, F, R>(&self, body: F) -> R
    where F: FnOnce(cell::Ref<Self::Item>) -> R;
}

impl<T> BorrowIn for cell::RefCell<T> {

    type Item = T;

    fn borrow_in<'r, F, R>(&self, body: F) -> R
    where F: FnOnce(cell::Ref<Self::Item>) -> R {
        let borrowed = self.borrow();
        body(borrowed)
    }
}

pub trait BorrowMutIn {

    type Item;

    fn borrow_mut_in<'r, F, R>(&self, body: F) -> R
    where F: FnOnce(cell::RefMut<Self::Item>) -> R;
}

impl<T> BorrowMutIn for cell::RefCell<T> {

    type Item = T;

    fn borrow_mut_in<'r, F, R>(&self, body: F) -> R
    where F: FnOnce(cell::RefMut<Self::Item>) -> R {
        let borrowed = self.borrow_mut();
        body(borrowed)
    }
}

