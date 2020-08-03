use glib::clone;
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gtk::TextTag;

use crate::types::Member;

use crate::appop::AppOp;
use crate::widgets;

pub struct Autocomplete {
    entry: sourceview4::View,
    listbox: gtk::ListBox,
    popover: gtk::Popover,
    window: gtk::Window,
    highlighted_entry: Vec<String>,
    popover_position: Option<i32>,
    popover_search: Option<String>,
    popover_closing: bool,
    op: Arc<Mutex<AppOp>>,
}

impl Autocomplete {
    pub fn new(
        op: Arc<Mutex<AppOp>>,
        window: gtk::Window,
        msg_entry: sourceview4::View,
        popover: gtk::Popover,
        listbox: gtk::ListBox,
    ) -> Autocomplete {
        Autocomplete {
            entry: msg_entry,
            listbox,
            popover,
            window,
            highlighted_entry: vec![],
            popover_position: None,
            popover_search: None,
            popover_closing: false,
            op,
        }
    }

    pub fn connect(self) {
        let this: Rc<RefCell<Autocomplete>> = Rc::new(RefCell::new(self));

        let context = this.borrow().entry.get_style_context();
        if let Some(fg) = context.lookup_color("theme_selected_bg_color") {
            let color = gdk::RGBA {
                red: fg.red,
                green: fg.green,
                blue: fg.blue,
                alpha: 1.0,
            };

            let tag = TextTag::new(Some("alias-highlight"));
            tag.set_property_foreground_rgba(Some(&color));

            if let Some(buffer) = this.borrow().entry.get_buffer() {
                if let Some(tag_table) = buffer.get_tag_table() {
                    tag_table.add(&tag);
                }
            }
        }

        let window = &this.borrow().window;
        this.borrow()
            .popover
            .connect_closed(clone!(@weak window => move |_| {
                // Reenable Escape to change state
                if let Some(app) = window.get_application() {
                    app.set_accels_for_action("app.back", &["Escape"]);
                }
            }));

        let own = this.clone();
        this.borrow()
            .window
            .connect_button_press_event(move |_, _| {
                if own.borrow().popover_position.is_some() {
                    own.borrow_mut().autocomplete_enter();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });

        let own = this.clone();
        if let Some(buffer) = this.borrow().entry.get_buffer() {
            buffer.connect_property_cursor_position_notify(move |buffer| {
                if let Ok(item) = own.try_borrow() {
                    let start_iter = buffer.get_start_iter();
                    let end_iter = buffer.get_end_iter();

                    if let Some(input) = buffer.get_text(&start_iter, &end_iter, false) {
                        item.add_highlight(input.to_string());
                    }
                }
            });
        }

        let own = this.clone();
        if let Some(buffer) = this.borrow().entry.get_buffer() {
            buffer.connect_changed(move |buffer| {
                if let Ok(item) = own.try_borrow() {
                    let start_iter = buffer.get_start_iter();
                    let end_iter = buffer.get_end_iter();

                    if let Some(input) = buffer.get_text(&start_iter, &end_iter, false) {
                        item.add_highlight(input.to_string());
                    }
                }
            });
        }

        let own = this.clone();
        if let Some(buffer) = this.borrow().entry.get_buffer() {
            buffer.connect_delete_range(move |_, start_iter, end_iter| {
                let start = start_iter.get_offset();
                let end = end_iter.get_offset();

                if let Ok(mut item) = own.try_borrow_mut() {
                    if let Some(pos) = item.popover_position {
                        if end <= pos + 1 || (start <= pos && end > pos) {
                            item.autocomplete_enter();
                        }
                    }
                }
            });
        }

        let own = this.clone();
        this.borrow().entry.connect_key_release_event(move |_, k| {
            if let gdk::keys::constants::Escape = k.get_keyval() {
                if own.borrow().popover_position.is_some() {
                    own.borrow_mut().autocomplete_enter();
                    return Inhibit(true);
                }
            }
            Inhibit(false)
        });

        let own = this.clone();
        this.borrow().entry.connect_key_press_event(move |w, ev| {
            match ev.get_keyval() {
                gdk::keys::constants::BackSpace => {
                    if let Some(buffer) = w.get_buffer() {
                        let start = buffer.get_start_iter();
                        let end = buffer.get_end_iter();

                        match buffer.get_text(&start, &end, false) {
                            Some(ref t) if t == "" => {
                                own.borrow_mut().autocomplete_enter();
                            }
                            None => {
                                own.borrow_mut().autocomplete_enter();
                            }
                            _ => {}
                        }
                    }

                    return glib::signal::Inhibit(false);
                }
                /* Tab and Enter key */
                gdk::keys::constants::Tab | gdk::keys::constants::Return => {
                    if own.borrow().popover_position.is_some() {
                        let widget = {
                            own.borrow_mut().popover_closing = true;
                            own.borrow_mut().autocomplete_arrow(0)
                        };
                        if let Some(w) = widget {
                            let ev: &gdk::Event = ev;
                            let _ = w.emit("button-press-event", &[ev]);
                        }
                    } else if ev.get_keyval() != gdk::keys::constants::Tab {
                        return glib::signal::Inhibit(false);
                    }
                }
                /* Arrow key */
                gdk::keys::constants::Up => {
                    if own.borrow().popover_position.is_none() {
                        return glib::signal::Inhibit(false);
                    }

                    let widget = { own.borrow_mut().autocomplete_arrow(-1) };
                    if let Some(w) = widget {
                        let ev: &gdk::Event = ev;
                        let _ = w.emit("button-press-event", &[ev]);
                    }
                }
                /* Arrow key */
                gdk::keys::constants::Down => {
                    if own.borrow().popover_position.is_none() {
                        return glib::signal::Inhibit(false);
                    }

                    let widget = { own.borrow_mut().autocomplete_arrow(1) };

                    if let Some(w) = widget {
                        let ev: &gdk::Event = ev;
                        let _ = w.emit("button-press-event", &[ev]);
                    }
                }
                _ => return glib::signal::Inhibit(false),
            }
            glib::signal::Inhibit(true)
        });

        let own = this.clone();
        this.borrow().entry.connect_key_release_event(move |e, ev| {
            if let Some(buffer) = e.get_buffer() {
                let is_tab = ev.get_keyval() == gdk::keys::constants::Tab;

                let start = buffer.get_start_iter();
                let end = buffer.get_end_iter();
                let text = buffer
                    .get_text(&start, &end, false)
                    .map(|gstr| gstr.to_string());

                /* when closing popover with tab */
                {
                    if own.borrow().popover_closing {
                        own.borrow_mut().popover_closing = false;
                        return Inhibit(false);
                    }
                }
                /* allow popover opening with tab
                 * don't update popover when the input didn't change */
                if !is_tab {
                    if let Some(ref text) = text {
                        if let Some(ref old) = own.borrow().popover_search {
                            if text == old {
                                return Inhibit(false);
                            }
                        }
                    }
                }
                /* update the popover when closed and tab is released
                 * don't update the popover the arrow keys are pressed */
                if (is_tab && own.borrow().popover_position.is_none())
                    || (ev.get_keyval() != gdk::keys::constants::Up
                        && ev.get_keyval() != gdk::keys::constants::Down)
                {
                    own.borrow_mut().popover_search = text.clone();
                    if let Some(buffer) = e.get_buffer() {
                        let pos = buffer.get_property_cursor_position();

                        if let Some(text) = text.clone() {
                            let graphs = text.chars().collect::<Vec<char>>();

                            if pos as usize > graphs.len() {
                                return Inhibit(false);
                            }

                            let (p1, _) = graphs.split_at(pos as usize);
                            let first = p1.iter().collect::<String>();
                            if own.borrow().popover_position.is_none() {
                                if !is_tab {
                                    if let Some(at_pos) = first.rfind('@') {
                                        own.borrow_mut().popover_position = Some(at_pos as i32);
                                    }
                                } else if let Some(space_pos) =
                                    first.rfind(|c: char| c.is_whitespace())
                                {
                                    own.borrow_mut().popover_position = Some(space_pos as i32 + 1);
                                } else {
                                    own.borrow_mut().popover_position = Some(0);
                                }
                            }
                        }

                        if own.borrow().popover_position.is_some() {
                            let list = {
                                own.borrow()
                                    .autocomplete(text, buffer.get_property_cursor_position())
                            };
                            let widget_list = { own.borrow_mut().autocomplete_show_popover(list) };
                            for (alias, widget) in widget_list.iter() {
                                widget.connect_button_press_event(clone!(
                                @strong own,
                                @strong alias
                                => move |_, ev| {
                                    own.borrow_mut().autocomplete_insert(alias.clone());
                                    if ev.is::<gdk::EventKey>() {
                                        let ev = {
                                            let ev: &gdk::Event = ev;

                                            ev.clone()
                                                .downcast::<gdk::EventKey>()
                                                .unwrap()
                                        };
                                        /* Submit on enter */
                                        if ev.get_keyval() == gdk::keys::constants::Return
                                            || ev.get_keyval() == gdk::keys::constants::Tab
                                        {
                                            own.borrow_mut().autocomplete_enter();
                                        }
                                    }
                                    else if ev.is::<gdk::EventButton>() {
                                        own.borrow_mut().autocomplete_enter();
                                    }
                                    Inhibit(true)
                                }));
                            }
                        }
                    }
                }
            }

            Inhibit(false)
        });
    }

    pub fn autocomplete_insert(&mut self, alias: String) {
        if let Some(start_pos) = self.popover_position {
            if let Some(buffer) = self.entry.get_buffer() {
                if let Some(mark) = buffer.get_insert() {
                    let mut start_iter = buffer.get_iter_at_offset(start_pos as i32);
                    let mut end_iter = buffer.get_iter_at_mark(&mark);
                    buffer.delete(&mut start_iter, &mut end_iter);
                    buffer.insert(&mut start_iter, &alias);
                    buffer.place_cursor(&start_iter);
                }
            }

            /* highlight member inside the entry */
            /* we need to set the highlight here the first time
             * because the ui changes from others are blocked as long we hold the look */
            if let Some(buffer) = self.entry.get_buffer() {
                self.highlighted_entry.push(alias);

                let start_iter = buffer.get_start_iter();
                let end_iter = buffer.get_end_iter();

                if let Some(input) = buffer.get_text(&start_iter, &end_iter, false) {
                    self.add_highlight(input.to_string());
                }
            }
        }
    }

    pub fn autocomplete_enter(&mut self) -> bool {
        if let Some(buffer) = self.entry.get_buffer() {
            let start_iter = buffer.get_start_iter();
            let end_iter = buffer.get_end_iter();

            if let Some(input) = buffer.get_text(&start_iter, &end_iter, false) {
                self.add_highlight(input.to_string());
            }
        }

        self.popover_position = None;
        self.popover_search = None;
        let visible = self.popover.is_visible();
        self.popover.popdown();

        visible
    }

    pub fn add_highlight(&self, input: String) {
        let input = input.to_lowercase();

        if let Some(buffer) = self.entry.get_buffer() {
            let start_iter = buffer.get_start_iter();
            let end_iter = buffer.get_end_iter();
            buffer.remove_tag_by_name("alias-highlight", &start_iter, &end_iter);

            for alias in self
                .highlighted_entry
                .iter()
                .map(|alias| alias.to_lowercase())
            {
                for (index, text) in input.match_indices(&alias) {
                    let start_iter = buffer.get_iter_at_offset(index as i32);
                    let end_iter = buffer.get_iter_at_offset((index + text.len()) as i32);

                    buffer.apply_tag_by_name("alias-highlight", &start_iter, &end_iter);
                }
            }
        }
    }

    pub fn autocomplete_arrow(&mut self, direction: i32) -> Option<gtk::Widget> {
        let mut result = None;
        if let Some(row) = self.listbox.get_selected_row() {
            let index = row.get_index() + direction;
            if index >= 0 {
                let row = self.listbox.get_row_at_index(row.get_index() + direction);
                match row {
                    None => {
                        if let Some(row) = self.listbox.get_row_at_index(0) {
                            self.listbox.select_row(Some(&row));
                            result = Some(row.get_children().first()?.clone());
                        }
                    }
                    Some(row) => {
                        self.listbox.select_row(Some(&row));
                        result = Some(row.get_children().first()?.clone());
                    }
                };
            } else if let Some(row) = self.listbox.get_children().last() {
                if let Ok(row) = row.clone().downcast::<gtk::ListBoxRow>() {
                    self.listbox.select_row(Some(&row));
                    result = Some(row.get_children().first()?.clone());
                }
            }
        } else if let Some(row) = self.listbox.get_row_at_index(0) {
            self.listbox.select_row(Some(&row));
            result = Some(row.get_children().first()?.clone());
        }
        result
    }

    pub fn autocomplete_show_popover(
        &mut self,
        list: Vec<Member>,
    ) -> HashMap<String, gtk::EventBox> {
        for ch in self.listbox.get_children().iter() {
            self.listbox.remove(ch);
        }

        let mut widget_list: HashMap<String, gtk::EventBox> = HashMap::new();

        if !list.is_empty() {
            for m in list.iter() {
                let alias = &m
                    .alias
                    .clone()
                    .unwrap_or_default()
                    .trim_end_matches(" (IRC)")
                    .to_owned();
                let widget;
                {
                    let guard = self.op.lock().unwrap();
                    let mb = widgets::MemberBox::new(&m, &guard);
                    widget = mb.widget(true);
                }

                let w = widget.clone();
                let a = alias.clone();
                widget_list.insert(a, w);
                self.listbox.add(&widget);
            }

            self.popover.set_relative_to(Some(&self.entry));
            self.popover
                .set_pointing_to(&self.entry.get_cursor_locations(None).0);
            self.popover.set_modal(false);

            if let Some(row) = self.listbox.get_row_at_index(0) {
                self.listbox.select_row(Some(&row));
            }

            self.popover.popup();
            // Don't change app state on Escape while the popover is open
            if let Some(app) = self.window.get_application() {
                app.set_accels_for_action("app.back", &[]);
            }
        } else {
            self.autocomplete_enter();
        }

        widget_list
    }

    pub fn autocomplete(&self, text: Option<String>, pos: i32) -> Vec<Member> {
        let mut list: Vec<Member> = vec![];
        let guard = self.op.lock().unwrap();
        let rooms = &guard.rooms;
        match text {
            None => {}
            Some(txt) => {
                if let Some(at_pos) = self.popover_position {
                    let last = {
                        let start = at_pos as usize;
                        let end = pos as usize;
                        txt.get(start..end)
                    };
                    if let Some(last) = last {
                        info!("Matching string '{}'", last);
                        /*remove @ from string*/
                        let w = if last.starts_with('@') {
                            last[1..].to_lowercase()
                        } else {
                            last.to_lowercase()
                        };

                        /* Search for the 5 most recent active users */
                        if let Some(aroom) = guard.active_room.clone() {
                            if let Some(r) = rooms.get(&aroom) {
                                let mut count = 0;
                                for (_, m) in r.members.iter() {
                                    let alias = &m.alias.clone().unwrap_or_default().to_lowercase();
                                    let uid = m.uid.localpart().to_lowercase();
                                    if alias.starts_with(&w) || uid.starts_with(&w) {
                                        list.push(m.clone());
                                        count += 1;
                                        /* Search only for 5 matching users */
                                        if count > 4 {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        list
    }
}
