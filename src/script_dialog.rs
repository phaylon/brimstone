
use webkit2gtk;

use app;

pub fn connect(app: &app::Handle, webview: &webkit2gtk::WebView) {
    use webkit2gtk::{ WebViewExt };

    webview.connect_script_dialog(with_cloned!(app, move |_webview, _dialog| {
        let _app = &app;
        println!("DIALOG");

        false
    }));
}
