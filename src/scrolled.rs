
use gtk;

pub fn create<W>(widget: W) -> gtk::ScrolledWindow
where W: gtk::IsA<gtk::Widget> {
    use gtk::{ ScrolledWindowExt };

    let scrolled_window = gtk::ScrolledWindow::new(None, None);
    scrolled_window.add_with_viewport(&widget);

    scrolled_window
}

