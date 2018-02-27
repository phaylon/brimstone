
#[macro_use] extern crate webkit2gtk_webextension;

extern crate url;

extern crate brimstone_domain_settings as domain_settings;
extern crate brimstone_page_state as page_state;

use std::rc;

web_extension_init_with_data!();

pub fn web_extension_initialize(
    extension: &webkit2gtk_webextension::WebExtension,
    _user_data: &glib::variant::Variant,
) {
    use webkit2gtk_webextension::{ WebExtensionExt, WebPageExt, URIRequestExt };

    let domains = domain_settings::Settings::open("_profile/config/domain_settings.db")
        .map(rc::Rc::new)
        .unwrap();

    let state = page_state::State::open_or_create("_profile/runtime/page_state.db")
        .map(rc::Rc::new)
        .unwrap();

    extension.connect_page_created({
        let domains = domains.clone();
        let state = state.clone();
        move |_extension, page| {

            page.connect_send_request({
                let domains = domains.clone();
                let state = state.clone();
                move |page, request, _redir_response| {
                    
                    let source_uri = page.get_uri();
                    let target_uri = request.get_uri();

                    let source_host = source_uri
                        .as_ref()
                        .and_then(|uri| parse_uri(uri))
                        .and_then(|uri| domain_settings::Host::from_uri(&uri));

                    let target_host = target_uri
                        .as_ref()
                        .and_then(|uri| parse_uri(uri))
                        .and_then(|uri| domain_settings::Host::from_uri(&uri));

                    let (source_host, target_host) = match (source_host, target_host) {
                        (Some(source_host), Some(target_host)) => (source_host, target_host),
                        _ => return false,
                    };

                    if source_host.is_related_to(&target_host) {
                        return false;
                    }

                    let handle = state.handle(page.get_id(), &source_host);
                    let allowed = domains.can_request(&source_host, &target_host);

                    if allowed {
                        handle.push_allowed(&target_host);
                        false
                    } else {
                        handle.push_denied(&target_host);
                        true
                    }
                }
            });
        }
    });
}

fn parse_uri(uri: &str) -> Option<url::Url> {
    match url::Url::parse(uri) {
        Ok(uri) => Some(uri),
        Err(err) => {
            eprintln!("URI parse error: {}", err);
            return None;
        },
    }
}
