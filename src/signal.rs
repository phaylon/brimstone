
use std::cell;

pub struct Notifier<S, T> where T: ?Sized {
    handlers: cell::RefCell<Vec<Box<Fn(&S, &T)>>>,
    emitting: cell::Cell<bool>,
}

impl<S, T> Notifier<S, T> where T: ?Sized {

    pub fn new() -> Notifier<S, T> {
        Notifier {
            handlers: cell::RefCell::new(Vec::new()),
            emitting: cell::Cell::new(false),
        }
    }

    pub fn connect<F>(&self, callback: F)
    where F: Fn(&S, &T) + 'static {
        self.handlers.borrow_mut().push(Box::new(callback));
    }

    pub fn emit(&self, object: &S, data: &T) {
        use dynamic::{ BorrowIn };

        if self.emitting.get() {
            panic!("notifier signal is already being emitted");
        }
        self.emitting.set(true);
        self.handlers.borrow_in(|handlers| {
            for handler in handlers.iter() {
                handler(object, data);
            }
        });
        self.emitting.set(false);
    }
}
