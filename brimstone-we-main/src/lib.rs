
#[macro_use] extern crate webkit2gtk_webextension;

extern crate serde;
extern crate serde_json;
extern crate url;

extern crate brimstone_domain_settings as domain_settings;
extern crate brimstone_page_state as page_state;

use std::rc;

web_extension_init_with_data!();

pub fn web_extension_initialize(
    extension: &webkit2gtk_webextension::WebExtension,
    user_data: &glib::variant::Variant,
) {
    use webkit2gtk_webextension::{ WebExtensionExt, WebPageExt, URIRequestExt };

    let user_data = user_data.get_str().expect("web extension initialization data");
    let init_args: page_state::InitArguments = serde_json::from_str(user_data)
        .expect("web process initialization arguments deserialization");

    let page_state_client = rc::Rc::new(page_state::Client::new(&init_args.instance));

    let domains = domain_settings::Settings::open(&init_args.domain_settings_path)
        .map(rc::Rc::new)
        .unwrap();

    extension.connect_page_created({
        let domains = domains.clone();
        let page_state_client = page_state_client.clone();
        move |_extension, page| {

            page.connect_send_request({
                let domains = domains.clone();
                let page_state_client = page_state_client.clone();
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

                    let allowed = domains.can_request(&source_host, &target_host);

                    page_state_client.push(page.get_id(), &source_host, &target_host, allowed);
                    !allowed
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
