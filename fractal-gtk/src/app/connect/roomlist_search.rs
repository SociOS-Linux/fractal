use glib::clone;
use gtk::prelude::*;

use crate::app::App;

impl App {
    pub fn connect_roomlist_search(&self) {
        let op = &self.op;

        let search_btn = self
            .ui
            .builder
            .get_object::<gtk::ToggleButton>("room_search_button")
            .expect("Can't find room_search_button in ui file.");
        let search_bar = self
            .ui
            .builder
            .get_object::<gtk::SearchBar>("room_list_searchbar")
            .expect("Can't find room_list_searchbar in ui file.");
        let search_entry = self
            .ui
            .builder
            .get_object::<gtk::SearchEntry>("room_list_search")
            .expect("Can't find room_list_search in ui file.");

        search_btn.connect_toggled(clone!(@strong search_bar => move |btn| {
            search_bar.set_search_mode(btn.get_active());
        }));

        search_bar.connect_property_search_mode_enabled_notify(
            clone!(@strong search_btn => move |headerbar| {
                search_btn.set_active(headerbar.get_search_mode());
            }),
        );

        search_entry.connect_search_changed(clone!(@strong op => move |entry| {
            op.lock().unwrap().filter_rooms(
                Some(entry.get_text().to_string())
            );
        }));

        // hidding left and right boxes to align with top buttons
        let boxes = search_bar.get_children()[0]
            .clone()
            .downcast::<gtk::Revealer>()
            .unwrap() // revealer
            .get_children()[0]
            .clone()
            .downcast::<gtk::Box>()
            .unwrap(); // box
        boxes.get_children()[0]
            .clone()
            .downcast::<gtk::Box>()
            .unwrap()
            .hide();
        boxes.get_children()[1].clone().set_hexpand(true);
        boxes.get_children()[1].clone().set_halign(gtk::Align::Fill);
        boxes.get_children()[2]
            .clone()
            .downcast::<gtk::Box>()
            .unwrap()
            .hide();
    }
}
