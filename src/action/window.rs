
use app;

const APP_NAME: &str = "Brimstone";

pub struct SetTitle<'title, 'url> {
    pub title: Option<&'title str>,
    pub uri: &'url str,
}

impl<'title, 'url> app::Perform for SetTitle<'title, 'url> {

    type Result = ();

    fn perform(self, app: &app::Handle) {
        use gtk::{ GtkWindowExt };

        let window = try_extract!(app.window());
        window.set_title(&match self.title {
            Some(title) =>
                if title.is_empty() {
                    format!("{} - {}", self.uri, APP_NAME)
                } else {
                    format!("{} - {}", title, APP_NAME)
                },
            None => format!("{} - {}", self.uri, APP_NAME),
        });
    }
}
