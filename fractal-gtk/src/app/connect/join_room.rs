use glib::clone;
use gtk::prelude::*;

use crate::app::App;

impl App {
    pub fn connect_join_room_dialog(&self) {
        let dialog = self
            .ui
            .builder
            .get_object::<libhandy::Dialog>("join_room_dialog")
            .expect("Can't find join_room_dialog in ui file.");
        let cancel = self
            .ui
            .builder
            .get_object::<gtk::Button>("cancel_join_room")
            .expect("Can't find cancel_join_room in ui file.");
        let confirm = self
            .ui
            .builder
            .get_object::<gtk::Button>("join_room_button")
            .expect("Can't find join_room_button in ui file.");
        let entry = self
            .ui
            .builder
            .get_object::<gtk::Entry>("join_room_name")
            .expect("Can't find join_room_name in ui file.");

        cancel.connect_clicked(clone!(@strong entry, @strong dialog => move |_| {
            dialog.hide();
            entry.set_text("");
        }));
        dialog.connect_delete_event(clone!(@strong entry, @strong dialog => move |_, _| {
            dialog.hide();
            entry.set_text("");
            glib::signal::Inhibit(true)
        }));

        let op = self.op.clone();
        confirm.connect_clicked(clone!(@strong entry, @strong dialog => move |_| {
            dialog.hide();
            op.lock().unwrap().join_to_room();
            entry.set_text("");
        }));

        let op = self.op.clone();
        entry.connect_activate(clone!(@strong dialog => move |entry| {
            dialog.hide();
            op.lock().unwrap().join_to_room();
            entry.set_text("");
        }));
        entry.connect_changed(clone!(@strong confirm => move |entry| {
                confirm.set_sensitive(entry.get_buffer().get_length() > 0);
        }));
    }
}
