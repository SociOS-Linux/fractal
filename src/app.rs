use std::{self, env, thread};
use std::time::Duration;

use futures;
use gio;
use gtk;
use gtk::prelude::*;
use bg_thread;
use url::Url;

// TODO: Is this the correct format for GApplication IDs?
const APP_ID: &'static str = "jplatte.ruma_gtk";

/// State for the main thread.
///
/// It takes care of starting up the application and for loading and accessing the
/// UI.
pub struct App {
    /// GTK Application which runs the main loop.
    gtk_app: gtk::Application,

    /// Used to access the UI elements.
    gtk_builder: gtk::Builder,

    /// Sender to the homeserver channel.
    ///
    /// With this channel, we tell the background thread to connect to a new
    /// matrix homeserver. Currently, there can only be one connection to a
    /// homeserver at one time, so sending a homeserver url over this channel
    /// will result in the background thread first disconnecting, if it is
    /// connected to another homeserver already.
    homeserver_chan_tx: futures::sync::mpsc::Sender<Url>,

    /// Channel receiver which allows to run actions from the matrix connection thread.
    ///
    /// Long polling is required to receive messages from the rooms and so they have to
    /// run in separate threads.  In order to allow those threads to modify the gtk content,
    /// they will send closures to the main thread using this channel.
    dispatch_chan_rx: std::sync::mpsc::Receiver<Box<Fn(&gtk::Builder) + Send>>,

    /// Matrix communication thread join handler used to clean up the tread when
    /// closing the application.
    bg_thread_join_handle: thread::JoinHandle<()>,
}

impl App {
    /// Create an App instance
    pub fn new() -> App {
        let gtk_app = gtk::Application::new(Some(APP_ID), gio::ApplicationFlags::empty())
            .expect("Failed to initialize GtkApplication");

        let gtk_builder = gtk::Builder::new_from_file("res/main_window.glade");

        gtk_app.connect_activate(clone!(gtk_builder => move |app| {
            // Set up shutdown callback
            let window: gtk::Window = gtk_builder.get_object("main_window")
                .expect("Couldn't find main_window in ui file.");

            window.connect_delete_event(clone!(app => move |_, _| {
                app.quit();
                Inhibit(false)
            }));

            // Set up user popover
            let user_button: gtk::Button = gtk_builder.get_object("user_button")
                .expect("Couldn't find user_button in ui file.");

            let user_menu: gtk::Popover = gtk_builder.get_object("user_menu")
                .expect("Couldn't find user_menu in ui file.");

            user_button.connect_clicked(move |_| user_menu.show());

            // Associate window with the Application and show it
            window.set_application(Some(app));
            window.show_all();
        }));

        let (homeserver_chan_tx, homeserver_chan_rx) = futures::sync::mpsc::channel(1);

        // Create channel to allow the matrix connection thread to send closures to the main loop.
        let (dispatch_chan_tx, dispatch_chan_rx) = std::sync::mpsc::channel();

        let bg_thread_join_handle =
            thread::spawn(move || bg_thread::run(homeserver_chan_rx, dispatch_chan_tx));

        App {
            gtk_app,
            gtk_builder,
            homeserver_chan_tx,
            dispatch_chan_rx,
            bg_thread_join_handle,
        }
    }

    pub fn run(self) {
        // Convert the args to a Vec<&str>. Application::run requires argv as &[&str]
        // and envd::args() returns an iterator of Strings.
        let args = env::args().collect::<Vec<_>>();
        let args_refs = args.iter().map(|x| &x[..]).collect::<Vec<_>>();

        // Poll the matrix communication thread channel and run the closures to allow
        // the threads to run actions in the main loop.
        let dispatch_chan_rx = self.dispatch_chan_rx;
        let gtk_builder = self.gtk_builder;
        gtk::idle_add(move || {
            if let Ok(dispatch_fn) = dispatch_chan_rx.recv_timeout(Duration::from_millis(5)) {
                dispatch_fn(&gtk_builder);
            }

            Continue(true)
        });

        // Run the main loop.
        self.gtk_app.run(args_refs.len() as i32, &args_refs);

        // Clean up
        self.bg_thread_join_handle.join().unwrap();
    }
}
