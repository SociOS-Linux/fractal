use crate::backend::{room, HandleError};
use fractal_api::identifiers::UserId;
use fractal_api::r0::AccessToken;
use glib::clone;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

use crate::i18n::ni18n_f;
use fractal_api::url::Url;
use gio::prelude::*;
use gtk::prelude::*;

use crate::actions;
use crate::actions::{ButtonState, StateExt};
use crate::app::App;
use crate::types::Member;
use crate::types::Room;
use crate::util::markup_text;
use crate::widgets;
use crate::widgets::avatar::AvatarExt;
use crate::widgets::members_list::MembersList;

#[derive(Debug, Clone)]
pub struct RoomSettings {
    actions: gio::SimpleActionGroup,
    room: Room,
    uid: UserId,
    builder: gtk::Builder,
    members_list: Option<MembersList>,
    server_url: Url,
    access_token: AccessToken,
}

impl RoomSettings {
    pub fn new(
        window: &gtk::Window,
        uid: UserId,
        room: Room,
        server_url: Url,
        access_token: AccessToken,
    ) -> RoomSettings {
        let builder = gtk::Builder::new();

        builder
            .add_from_resource("/org/gnome/Fractal/ui/room_settings.ui")
            .expect("Can't load ui file: room_settings.ui");

        let stack = builder
            .get_object::<gtk::Stack>("room_settings_stack")
            .expect("Can't find room_settings_stack in ui file.");

        let actions = actions::RoomSettings::new(&window, server_url.clone(), access_token.clone());
        stack.insert_action_group("room-settings", Some(&actions));

        RoomSettings {
            actions,
            room,
            uid,
            builder,
            members_list: None,
            server_url,
            access_token,
        }
    }

    /* creates a empty list with members.len() rows, the content will be loaded when the row is
     * drawn */
    pub fn create(&mut self) -> Option<(gtk::Box, gtk::Box)> {
        let body = self
            .builder
            .get_object::<gtk::Box>("room_settings_box")
            .expect("Can't find room_settings_box in ui file.");
        let header = self
            .builder
            .get_object::<gtk::Box>("room_settings_headerbar")
            .expect("Can't find room_settings_headerbar in ui file.");
        let stack = self
            .builder
            .get_object::<gtk::Stack>("room_settings_stack")
            .expect("Can't find room_settings_stack in ui file.");

        // We can have rooms without name or topic but with members, the 1:1 rooms are this, so
        // we should show the loading if we've nothing, if there's something we need to show
        // the info
        if self.room.avatar.is_none() && self.room.topic.is_none() && self.room.members.is_empty() {
            stack.set_visible_child_name("loading")
        } else {
            stack.set_visible_child_name("info")
        }

        self.init_room_settings();
        self.connect();

        Some((body, header))
    }

    #[allow(dead_code)]
    pub fn update_members_list(&self, uid: UserId) -> Option<()> {
        self.members_list.clone()?.update(uid);
        None
    }

    pub fn connect(&self) {
        let name_btn = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_name_button")
            .expect("Can't find room_settings_room_name_button in ui file.");
        let name_entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_name_entry")
            .expect("Can't find room_settings_room_name_entry in ui file.");
        let topic_btn = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_topic_button")
            .expect("Can't find room_settings_room_topic_button in ui file.");
        let topic_entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_topic_entry")
            .expect("Can't find room_settings_room_topic_entry in ui file.");
        let avatar_btn = self
            .builder
            .get_object::<gtk::Button>("room_settings_avatar_button")
            .expect("Can't find room_settings_avatar_button in ui file.");

        let this: Rc<RefCell<RoomSettings>> = Rc::new(RefCell::new(self.clone()));

        let button = name_btn.clone();
        name_entry.connect_property_text_notify(clone!(@strong this => move |w| {
            let result = this.borrow().validate_room_name(
                Some(w.get_text().to_string())
            );
            button.set_visible(result.is_some());
        }));

        let button = topic_btn.clone();
        topic_entry.connect_property_text_notify(clone!(@strong this => move |w| {
            let result = this.borrow().validate_room_topic(
                Some(w.get_text().to_string())
            );
            button.set_visible(result.is_some());
        }));

        // TODO: create actions for all button
        let button = name_btn.clone();
        name_entry.connect_activate(move |_w| {
            let _ = button.emit("clicked", &[]);
        });

        name_btn.connect_clicked(clone!(@strong this => move |_| {
            this.borrow_mut().update_room_name();
        }));

        let button = topic_btn.clone();
        topic_entry.connect_activate(move |_w| {
            let _ = button.emit("clicked", &[]);
        });

        topic_btn.connect_clicked(clone!(@strong this => move |_| {
            this.borrow_mut().update_room_topic();
        }));

        if let Some(action) = self.actions.lookup_action("change-avatar") {
            action.bind_button_state(&avatar_btn);
            let data = glib::Variant::from(&self.room.id.to_string());
            avatar_btn.set_action_target_value(Some(&data));
            avatar_btn.set_action_name(Some("room-settings.change-avatar"));
            let avatar_spinner = self
                .builder
                .get_object::<gtk::Spinner>("room_settings_avatar_spinner")
                .expect("Can't find room_settings_avatar_spinner in ui file.");
            avatar_btn.connect_property_sensitive_notify(
                clone!(@weak avatar_spinner as spinner => move |w| {
                    if w.get_sensitive() {
                        spinner.hide();
                        spinner.stop();
                    } else {
                        spinner.start();
                        spinner.show();
                    }
                }),
            );
        }
    }

    fn init_room_settings(&mut self) {
        let name = self.room.name.clone();
        let topic = self.room.topic.clone();
        let mut is_room = true;
        let mut is_group = false;
        let members: Vec<Member> = self.room.members.values().cloned().collect();
        let power = self.room.admins.get(&self.uid).copied().unwrap_or(0);

        let edit = power >= 50 && !self.room.direct;

        let description = if self.room.direct {
            is_room = false;
            is_group = false;
            self.get_direct_partner_uid(members.clone())
                .as_ref()
                .map(ToString::to_string)
        } else {
            /* we don't have private groups yet
            let description = Some(format!("Private Group · {} members", members.len()));
            */
            //Some(format!("Public Room · {} members", members.len()))

            Some(ni18n_f(
                "Room · {} member",
                "Room · {} members",
                members.len() as u32,
                &[&members.len().to_string()],
            ))
        };

        self.room_settings_show_avatar(edit);
        self.room_settings_show_room_name(name, edit);
        self.room_settings_show_room_topic(topic, is_room, edit);
        self.room_settings_show_room_type(description);
        self.room_settings_show_members(members);

        /* admin parts */
        self.room_settings_show_group_room(is_room || is_group);
        self.room_settings_show_admin_groupe(is_group && edit);
        self.room_settings_show_admin_room(is_room && edit);
        self.room_settings_hide_not_implemented_widgets();
    }

    /* returns the uid of the fisrt member in the room, ignoring the current user */
    fn get_direct_partner_uid(&self, members: Vec<Member>) -> Option<UserId> {
        members
            .iter()
            .map(|m| m.uid.clone())
            .find(|uid| *uid != self.uid)
    }

    pub fn room_settings_show_room_name(&self, text: Option<String>, edit: bool) -> Option<()> {
        let label = self
            .builder
            .get_object::<gtk::Label>("room_settings_room_name")
            .expect("Can't find room_settings_room_name in ui file.");
        let b = self
            .builder
            .get_object::<gtk::Box>("room_settings_room_name_box")
            .expect("Can't find room_settings_room_topic_entry in ui file.");
        let entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_name_entry")
            .expect("Can't find room_settings_room_name_entry in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_name_button")
            .expect("Can't find room_settings_room_name_button in ui file.");

        if edit {
            if let Some(text) = text {
                entry.set_text(&text);
            } else {
                entry.set_text("");
            }
            label.hide();
            entry.set_editable(true);
            self.reset_action_button(button);
            b.show();
        } else {
            if let Some(text) = text {
                label.set_text(&text);
            } else {
                label.set_text("");
            }
            b.hide();
            label.show();
        }
        None
    }

    pub fn reset_action_button(&self, button: gtk::Button) {
        let image = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
        button.set_image(Some(&image));
        button.set_sensitive(true);
    }

    pub fn room_settings_show_room_topic(
        &self,
        text: Option<String>,
        is_room: bool,
        edit: bool,
    ) -> Option<()> {
        let label = self
            .builder
            .get_object::<gtk::Label>("room_settings_room_topic")
            .expect("Can't find room_settings_room_topic in ui file.");
        let b = self
            .builder
            .get_object::<gtk::Box>("room_settings_room_topic_box")
            .expect("Can't find room_settings_room_topic_entry in ui file.");
        let entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_topic_entry")
            .expect("Can't find room_settings_room_topic_entry in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_topic_button")
            .expect("Can't find room_settings_room_topic_button in ui file.");

        if is_room {
            if edit {
                if let Some(text) = text {
                    entry.set_text(&text);
                } else {
                    entry.set_text("");
                }
                label.hide();
                entry.set_editable(true);
                self.reset_action_button(button);
                b.show();
            } else {
                b.hide();
                if let Some(text) = text {
                    let m = markup_text(&text);
                    label.set_markup(&m);
                    label.show();
                } else {
                    label.hide();
                }
            }
        } else {
            b.hide();
            label.hide();
        }

        None
    }

    pub fn room_settings_show_group_room(&self, show: bool) -> Option<()> {
        let notify = self
            .builder
            .get_object::<gtk::Frame>("room_settings_notification_sounds")
            .expect("Can't find room_settings_notification_sounds in ui file.");
        let invite = self
            .builder
            .get_object::<gtk::Button>("room_settings_invite")
            .expect("Can't find room_settings_invite in ui file.");

        if show {
            notify.show();
            invite.show();
        } else {
            notify.hide();
            invite.hide();
        }

        None
    }

    pub fn room_settings_show_admin_groupe(&self, show: bool) -> Option<()> {
        let history = self
            .builder
            .get_object::<gtk::Frame>("room_settings_history_visibility")
            .expect("Can't find room_settings_history_visibility in ui file.");

        if show {
            history.show();
        } else {
            history.hide();
        }

        None
    }

    pub fn room_settings_show_admin_room(&self, show: bool) -> Option<()> {
        let room = self
            .builder
            .get_object::<gtk::Frame>("room_settings_room_visibility")
            .expect("Can't find room_settings_room_visibility in ui file.");
        let join = self
            .builder
            .get_object::<gtk::Frame>("room_settings_join")
            .expect("Can't find room_settings_join in ui file.");

        if show {
            room.show();
            join.show();
        } else {
            room.hide();
            join.hide();
        }

        None
    }

    pub fn room_settings_show_room_type(&self, text: Option<String>) -> Option<()> {
        let label = self
            .builder
            .get_object::<gtk::Label>("room_settings_room_description")
            .expect("Can't find room_settings_room_name in ui file.");
        label.set_selectable(true);

        if let Some(text) = text {
            label.set_text(&text);
            label.show();
        } else {
            label.hide();
        }
        None
    }

    fn room_settings_show_avatar(&self, edit: bool) {
        let container = self
            .builder
            .get_object::<gtk::Box>("room_settings_avatar_box")
            .expect("Can't find room_settings_avatar_box in ui file.");
        let avatar_btn = self
            .builder
            .get_object::<gtk::Button>("room_settings_avatar_button")
            .expect("Can't find room_settings_avatar_button in ui file.");

        for w in container.get_children().iter() {
            if w != &avatar_btn {
                container.remove(w);
            }
        }

        let server = self.server_url.clone();
        let access_token = self.access_token.clone();
        let room_id = self.room.id.clone();
        thread::spawn(
            move || match room::get_room_avatar(server, access_token, room_id) {
                Ok((room, avatar)) => {
                    APPOP!(set_room_avatar, (room, avatar));
                }
                Err(err) => {
                    err.handle_error();
                }
            },
        );
        let image = widgets::Avatar::avatar_new(Some(100));
        let _data = image.circle(
            self.room.id.to_string(),
            self.room.name.clone(),
            100,
            None,
            None,
        );

        if edit {
            let overlay = self
                .builder
                .get_object::<gtk::Overlay>("room_settings_avatar_overlay")
                .expect("Can't find room_settings_avatar_overlay in ui file.");
            let overlay_box = self
                .builder
                .get_object::<gtk::Box>("room_settings_avatar")
                .expect("Can't find room_settings_avatar in ui file.");
            let avatar_spinner = self
                .builder
                .get_object::<gtk::Spinner>("room_settings_avatar_spinner")
                .expect("Can't find room_settings_avatar_spinner in ui file.");
            /* remove all old avatar */
            for w in overlay_box.get_children().iter() {
                overlay_box.remove(w);
            }
            overlay_box.add(&image);
            overlay.show();
            avatar_spinner.hide();
            avatar_btn.set_sensitive(true);
            /*Hack for button bug */
            avatar_btn.hide();
            avatar_btn.show();
        } else {
            avatar_btn.hide();
            container.add(&image);
        }
    }

    pub fn update_room_name(&mut self) -> Option<()> {
        let entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_name_entry")
            .expect("Can't find room_settings_name_entry in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_name_button")
            .expect("Can't find room_settings_name_button in ui file.");

        let new_name = entry.get_text().to_string();
        let room = &self.room;

        let spinner = gtk::Spinner::new();
        spinner.start();
        button.set_image(Some(&spinner));
        button.set_sensitive(false);
        entry.set_editable(false);

        let server = self.server_url.clone();
        let access_token = self.access_token.clone();
        let room_id = room.id.clone();
        thread::spawn(
            move || match room::set_room_name(server, access_token, room_id, new_name) {
                Ok(_) => {
                    APPOP!(show_new_room_name);
                }
                Err(err) => {
                    err.handle_error();
                }
            },
        );

        None
    }

    pub fn validate_room_name(&self, new_name: Option<String>) -> Option<String> {
        let room = &self.room;
        let old_name = room.name.clone()?;
        let new_name = new_name?;
        if new_name != "" && new_name != old_name {
            return Some(new_name);
        }

        None
    }

    pub fn validate_room_topic(&self, new_name: Option<String>) -> Option<String> {
        let room = &self.room;
        let old_name = room.topic.clone()?;
        let new_name = new_name?;
        if new_name != "" && new_name != old_name {
            return Some(new_name);
        }

        None
    }

    pub fn update_room_topic(&mut self) -> Option<()> {
        let name = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_topic_entry")
            .expect("Can't find room_settings_topic in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_topic_button")
            .expect("Can't find room_settings_topic_button in ui file.");
        let topic = name.get_text().to_string();

        let room = &self.room;

        let spinner = gtk::Spinner::new();
        spinner.start();
        button.set_image(Some(&spinner));
        button.set_sensitive(false);
        name.set_editable(false);

        let server = self.server_url.clone();
        let access_token = self.access_token.clone();
        let room_id = room.id.clone();
        thread::spawn(
            move || match room::set_room_topic(server, access_token, room_id, topic) {
                Ok(_) => {
                    APPOP!(show_new_room_topic);
                }
                Err(err) => {
                    err.handle_error();
                }
            },
        );

        None
    }

    pub fn show_new_room_avatar(&self) {
        if let Some(action) = self.actions.lookup_action("change-avatar") {
            action.change_state(&ButtonState::Sensitive.into());
        }
    }

    pub fn show_new_room_name(&self) {
        let entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_name_entry")
            .expect("Can't find room_settings_room_name_entry in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_name_button")
            .expect("Can't find room_settings_name_button in ui file.");
        button.hide();
        entry.set_editable(true);
        self.reset_action_button(button);
    }

    pub fn show_new_room_topic(&self) {
        let entry = self
            .builder
            .get_object::<gtk::Entry>("room_settings_room_topic_entry")
            .expect("Can't find room_settings_room_topic_entry in ui file.");
        let button = self
            .builder
            .get_object::<gtk::Button>("room_settings_room_topic_button")
            .expect("Can't find room_settings_topic_button in ui file.");
        button.hide();
        entry.set_editable(true);
        self.reset_action_button(button);
    }

    fn room_settings_hide_not_implemented_widgets(&self) -> Option<()> {
        let notification = self
            .builder
            .get_object::<gtk::Frame>("room_settings_notification_sounds")
            .expect("Can't find room_settings_notification_sounds in ui file.");
        let media = self
            .builder
            .get_object::<gtk::Frame>("room_settings_media")
            .expect("Can't find room_settings_media in ui file.");
        let switch = self
            .builder
            .get_object::<gtk::Frame>("room_settings_notification_switch")
            .expect("Can't find room_settings_notification_switch in ui file.");
        let history = self
            .builder
            .get_object::<gtk::Frame>("room_settings_history_visibility")
            .expect("Can't find room_settings_history_visibility in ui file.");
        let join = self
            .builder
            .get_object::<gtk::Frame>("room_settings_join")
            .expect("Can't find room_settings_join in ui file.");
        let room = self
            .builder
            .get_object::<gtk::Frame>("room_settings_room_visibility")
            .expect("Can't find room_settings_room_visibility in ui file.");
        notification.hide();
        media.hide();
        switch.hide();
        history.hide();
        room.hide();
        join.hide();

        None
    }

    fn room_settings_show_members(&mut self, members: Vec<Member>) -> Option<()> {
        let entry = self
            .builder
            .get_object::<gtk::SearchEntry>("room_settings_members_search")
            .expect("Can't find room_settings_members_search in ui file.");
        let b = self
            .builder
            .get_object::<gtk::Frame>("room_settings_members_list")
            .expect("Can't find room_settings_members_list in ui file.");
        let label = self
            .builder
            .get_object::<gtk::Label>("room_settings_member_list_title")
            .expect("Can't find room_settings_member_list_title in ui file.");
        for w in b.get_children().iter() {
            b.remove(w);
        }
        label.set_text(
            ni18n_f(
                "{} member",
                "{} members",
                members.len() as u32,
                &[&members.len().to_string()],
            )
            .as_str(),
        );
        let list = widgets::MembersList::new(members, self.room.admins.clone(), entry);
        let w = list.create()?;
        b.add(&w);
        self.members_list = Some(list);
        None
    }
}
