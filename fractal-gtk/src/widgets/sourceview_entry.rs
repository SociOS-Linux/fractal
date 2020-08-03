use gtk::prelude::*;
use libhandy::{Column, ColumnExt};
use sourceview4::ViewExt;
// This alias is necessary to avoid conflict with gtk's TextViewExt
use gspell::TextViewExt as GspellTextViewExt;

#[derive(Debug, Clone)]
pub struct SVEntry {
    pub column: Column,
    pub container: gtk::Box,
    pub attach: gtk::Button,
    pub markdown: gtk::MenuButton,
    pub markdown_img: gtk::Image,
    pub entry_box: gtk::Box,
    pub scroll: gtk::ScrolledWindow,
    pub view: sourceview4::View,
    pub buffer: sourceview4::Buffer,
    pub send: gtk::Button,
}

impl Default for SVEntry {
    fn default() -> Self {
        let column = Column::new();
        column.set_maximum_width(800);
        column.set_linear_growth_width(600);
        column.set_vexpand(false);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        container.set_vexpand(false);

        let size = gtk::IconSize::Button;
        let attach = gtk::Button::new();
        let attach_img = gtk::Image::from_icon_name(Some("mail-attachment-symbolic"), size);
        attach.set_image(Some(&attach_img));
        attach.set_valign(gtk::Align::End);
        attach.set_receives_default(true);
        attach.set_action_name(Some("app.send-file"));
        // TODO: there was an a11y object in the xml
        /*
        <object class="AtkObject" id="attach_button-atkobject">
          <property name="AtkObject::accessible-name" translatable="yes">Attach files</property>
        </object>
        */

        let markdown = gtk::MenuButton::new();
        let markdown_img = gtk::Image::from_icon_name(Some("format-justify-left-symbolic"), size);
        markdown.set_image(Some(&markdown_img));
        markdown.set_valign(gtk::Align::End);
        markdown.set_receives_default(true);
        // TODO: there was an a11y object in the xml
        /*
        <object class="AtkObject" id="a11y-markdown_button">
          <property name="AtkObject::accessible_name" translatable="yes">Text formatting</property>
        </object>
        */

        let entry_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        entry_box.get_style_context().add_class("message-input");

        let hadjust: Option<&gtk::Adjustment> = None;
        let vadjust: Option<&gtk::Adjustment> = None;
        let scroll = gtk::ScrolledWindow::new(hadjust, vadjust);

        let tag_table: Option<&gtk::TextTagTable> = None;
        let buffer = sourceview4::Buffer::new(tag_table);
        let view = sourceview4::View::with_buffer(&buffer);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        view.set_indent_on_tab(false);

        let textview = view.upcast_ref::<gtk::TextView>();
        let gspell_view = gspell::TextView::get_from_gtk_text_view(textview).unwrap();
        gspell_view.basic_setup();

        scroll.add(&view);
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);
        scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::External);
        scroll.set_max_content_height(100);
        scroll.set_propagate_natural_height(true);
        entry_box.add(&scroll);

        let send = gtk::Button::new();
        let send_img = gtk::Image::from_icon_name(Some("send-symbolic"), size);
        send.set_image(Some(&send_img));
        send.set_valign(gtk::Align::End);
        send.set_receives_default(true);
        send.get_style_context().add_class("suggested-action");
        send.set_action_name(Some("app.send-message"));

        container.pack_start(&attach, false, false, 0);
        container.pack_start(&markdown, false, false, 0);
        container.pack_start(&entry_box, false, true, 0);
        container.pack_start(&send, false, false, 0);

        column.add(&container);
        column.show_all();

        SVEntry {
            column,
            container,
            attach,
            markdown,
            markdown_img,
            entry_box,
            scroll,
            view,
            buffer,
            send,
        }
    }
}
