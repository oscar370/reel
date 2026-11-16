use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::ReelWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ReelApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for ReelApplication {
        const NAME: &'static str = "ReelApplication";
        type Type = super::ReelApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for ReelApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<control>q"]);
            obj.set_accels_for_action("win.next", &["Right"]);
            obj.set_accels_for_action("win.previous", &["Left"]);
        }
    }

    impl ApplicationImpl for ReelApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            // Get the current window or create one if necessary
            let window = self.obj().window();

            // Ask the window manager/compositor to present the window
            window.present();
        }

        fn open(&self, files: &[gio::File], _hint: &str) {
            let Some(file) = files.first() else {
                return;
            };

            let Some(path) = file.path() else {
                glib::g_warning!("reel", "Only local files are supported");
                return;
            };

            let window = self.obj().window();

            if let Err(error) = window.open_path(&path) {
                glib::g_warning!("reel", "Failed to open {}: {error}", path.display());
            }

            window.present();
        }
    }

    impl GtkApplicationImpl for ReelApplication {}
    impl AdwApplicationImpl for ReelApplication {}
}

glib::wrapper! {
    pub struct ReelApplication(ObjectSubclass<imp::ReelApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ReelApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/io/github/oscar370/Reel")
            .build()
    }

    fn window(&self) -> ReelWindow {
        self.active_window()
            .and_downcast::<ReelWindow>()
            .unwrap_or_else(|| ReelWindow::new(self))
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, about_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Reel")
            .application_icon("io.github.oscar370.Reel")
            .developer_name("oscar370")
            .version(VERSION)
            .developers(vec!["oscar370"])
            .website("https://github.com/oscar370/reel")
            .issue_url("https://github.com/oscar370/reel/issues")
            .license_type(gtk::License::Gpl30)
            // Translators: Replace "translator-credits" with your name/username, and optionally an email or URL.
            .translator_credits(gettext("translator-credits"))
            .copyright("© 2026 oscar370")
            .build();

        about.present(Some(&window));
    }
}
