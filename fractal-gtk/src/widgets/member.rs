use gtk::prelude::*;

use crate::types::Member;

use crate::appop::AppOp;

use crate::cache::download_to_cache;
use crate::globals;
use crate::widgets;
use crate::widgets::AvatarExt;

// Room Search item
pub struct MemberBox<'a> {
    member: &'a Member,
    op: &'a AppOp,
}

impl<'a> MemberBox<'a> {
    pub fn new(member: &'a Member, op: &'a AppOp) -> MemberBox<'a> {
        MemberBox { member, op }
    }

    pub fn widget(&self, show_uid: bool) -> gtk::EventBox {
        let username = gtk::Label::new(None);
        let uid = gtk::Label::new(None);
        let event_box = gtk::EventBox::new();
        let w = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 0);

        uid.set_text(&self.member.uid.to_string());
        uid.set_valign(gtk::Align::Start);
        uid.set_halign(gtk::Align::Start);
        uid.get_style_context().add_class("member-uid");

        username.set_text(&self.member.get_alias());
        let mut alias = self.member.get_alias();
        alias.push_str("\n");
        alias.push_str(&self.member.uid.to_string());
        username.set_tooltip_text(Some(&alias[..]));
        username.set_margin_end(5);
        username.set_ellipsize(pango::EllipsizeMode::End);
        username.set_valign(gtk::Align::Center);
        username.set_halign(gtk::Align::Start);
        username.get_style_context().add_class("member");

        let avatar = widgets::Avatar::avatar_new(Some(globals::USERLIST_ICON_SIZE));
        let badge = match self.op.member_level(self.member) {
            100 => Some(widgets::AvatarBadgeColor::Gold),
            50..=99 => Some(widgets::AvatarBadgeColor::Silver),
            _ => None,
        };
        let data = avatar.circle(
            self.member.uid.to_string(),
            Some(alias),
            globals::USERLIST_ICON_SIZE,
            badge,
            None,
        );
        if let Some(login_data) = self.op.login_data.clone() {
            download_to_cache(
                self.op.thread_pool.clone(),
                self.op.user_info_cache.clone(),
                login_data.server_url,
                login_data.access_token,
                self.member.uid.clone(),
                data,
            );
        };

        avatar.set_margin_start(3);
        avatar.set_valign(gtk::Align::Center);

        v.set_margin_start(3);
        v.pack_start(&username, true, true, 0);
        if show_uid {
            v.pack_start(&uid, true, true, 0);
        }

        w.add(&avatar);
        w.add(&v);

        event_box.add(&w);
        event_box.show_all();
        event_box
    }

    pub fn pill(&self) -> gtk::Box {
        let pill = gtk::Box::new(gtk::Orientation::Horizontal, 3);

        let username = gtk::Label::new(None);

        username.set_text(&self.member.get_alias());
        username.set_margin_end(3);
        username.get_style_context().add_class("msg-highlighted");

        let avatar = widgets::Avatar::avatar_new(Some(globals::PILL_ICON_SIZE));
        let data = avatar.circle(
            self.member.uid.to_string(),
            Some(self.member.get_alias()),
            globals::PILL_ICON_SIZE,
            None,
            None,
        );
        if let Some(login_data) = self.op.login_data.clone() {
            download_to_cache(
                self.op.thread_pool.clone(),
                self.op.user_info_cache.clone(),
                login_data.server_url,
                login_data.access_token,
                self.member.uid.clone(),
                data,
            );
        };

        avatar.set_margin_start(3);

        pill.pack_start(&avatar, true, true, 0);
        pill.pack_start(&username, true, true, 0);
        pill.show_all();
        pill
    }
}
