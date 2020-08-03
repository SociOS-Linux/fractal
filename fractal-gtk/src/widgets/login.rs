use fractal_api::url::Url;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use log::info;

use crate::actions;
use crate::actions::global::AppState;
use crate::actions::login::LoginState;
use crate::appop::AppOp;
use crate::globals;
use crate::i18n::i18n;
use crate::widgets::ErrorDialog;

use crate::backend::register::get_well_known;

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct LoginWidget {
    pub container: gtk::Stack,
    pub headers: gtk::Stack,
    pub server_entry: gtk::Entry,
    pub username_entry: gtk::Entry,
    pub password_entry: gtk::Entry,
    server_err_label: gtk::Label,
    credentials_err_label: gtk::Label,
    actions: gio::SimpleActionGroup,
}

impl LoginWidget {
    pub fn new(op: &Arc<Mutex<AppOp>>) -> Self {
        let widget = Self::default();

        let server_entry = &widget.server_entry;
        let username_entry = &widget.username_entry;
        let password_entry = &widget.password_entry;
        let err_label = &widget.credentials_err_label;

        // Grab the focus for each state
        widget
            .container
            .connect_property_visible_child_name_notify(clone!(
            @weak server_entry as server,
            @weak username_entry as username
            => move |container| {
                let state: LoginState = container
                    .get_visible_child_name()
                    .unwrap()
                    .to_string()
                    .into();

                match state {
                    LoginState::ServerChooser => server.grab_focus(),
                    LoginState::Credentials => username.grab_focus(),
                    _ => (),
                }
            }));

        let op = op.clone();

        let login = widget
            .actions
            .lookup_action("login")
            .expect("Could not find 'login' action for LoginWidget")
            .downcast::<gio::SimpleAction>()
            .expect("Could not cast action 'login' to SimpleAction");

        login.connect_activate(clone!(
        @weak server_entry,
        @weak username_entry,
        @weak password_entry,
        @weak err_label
        => move |_, _| {
            if let Some(txt) = server_entry.get_text() {
                let username = username_entry
                    .get_text()
                    .map_or(String::new(), |gstr| gstr.to_string());

                let password = password_entry
                    .get_text()
                    .map_or(String::new(), |gstr| gstr.to_string());

                let txt = String::from(txt).trim().to_string();
                let txt = if txt.starts_with("http://") || txt.starts_with("https://") {
                    txt
                } else {
                    format!("https://{}", &txt)
                };
                let txt = if !txt.ends_with('/') { txt + "/" } else { txt };

                if !password.is_empty() && !username.is_empty() {
                    // take the user's homeserver value if the
                    // well-known request fails
                    let homeserver_url = if let Ok(hs_url) = Url::parse(&txt) {
                        hs_url
                    } else {
                        let msg = i18n("Malformed server URL");
                        ErrorDialog::new(false, &msg);
                        return;
                    };

                    let (homeserver_url, idserver) = get_well_known(homeserver_url.clone())
                        .and_then(|response| {
                            let hs_url = Url::parse(&response.homeserver.base_url)?;
                            let ids = response
                                .identity_server
                                .as_ref()
                                .map(|ids| Url::parse(&ids.base_url))
                                .transpose()?
                                .unwrap_or(globals::DEFAULT_IDENTITYSERVER.clone());
                            info!("Got well-known response from {}: {:#?}", &txt, response);

                            Ok((hs_url, ids))
                        })
                        .map_err(|e| {
                            info!("Failed to .well-known request: {:#?}", e);
                            e
                        })
                        .unwrap_or((homeserver_url, globals::DEFAULT_IDENTITYSERVER.clone()));

                    err_label.hide();
                    op.lock().unwrap().set_state(AppState::Loading);
                    op.lock().unwrap().since = None;
                    op.lock()
                        .unwrap()
                        .connect(username, password, homeserver_url, idserver);
                } else {
                    err_label.show();
                }
            }
        }));

        let credentials = widget
            .actions
            .lookup_action("credentials")
            .expect("Could not find 'credentials' action for LoginWidget")
            .downcast::<gio::SimpleAction>()
            .expect("Could not cast action 'credentials' to SimpleAction");
        widget
            .server_entry
            .connect_activate(move |_| credentials.activate(None));

        widget
            .username_entry
            .connect_activate(clone!(@weak password_entry => move |_| {
                password_entry.grab_focus();
            }));

        widget
            .password_entry
            .connect_activate(move |_| login.activate(None));

        widget
    }
}

impl Default for LoginWidget {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Fractal/ui/login_flow.ui");

        let container: gtk::Stack = builder.get_object("login_flow_stack").unwrap();
        let headers: gtk::Stack = builder.get_object("login_flow_headers").unwrap();
        let server_entry = builder.get_object("server_chooser_entry").unwrap();
        let username_entry = builder.get_object("username_entry").unwrap();
        let password_entry = builder.get_object("password_entry").unwrap();

        let server_err_label = builder.get_object("server_err_label").unwrap();
        let credentials_err_label = builder.get_object("credentials_err_label").unwrap();

        let actions = actions::Login::new(&container, &headers, &server_entry, &server_err_label);

        container.show_all();
        headers.show_all();

        LoginWidget {
            container,
            headers,
            server_entry,
            username_entry,
            password_entry,
            server_err_label,
            credentials_err_label,
            actions,
        }
    }
}
