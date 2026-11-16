use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::errors::{generic_user_message, AppError};
use crate::navigator::{media_kind, FolderNavigator, MediaKind};

const CONTROLS_HIDE_DELAY: Duration = Duration::from_secs(2);

async fn load_first_frame(path: &Path) -> anyhow::Result<(glycin::Image, glycin::Frame)> {
    let file = gio::File::for_path(path);
    let mut image = glycin::Loader::new(file).load().await?;
    let frame = image.next_frame().await?;

    Ok((image, frame))
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/oscar370/Reel/window.ui")]
    pub struct ReelWindow {
        // Template widgets
        #[template_child]
        pub file_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub view_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub media_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub image_view: TemplateChild<gtk::Picture>,
        #[template_child]
        pub video_view: TemplateChild<gtk::Video>,
        #[template_child]
        pub error_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub open_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub header_open_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub prev_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub next_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub prev_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_button: TemplateChild<gtk::Button>,

        pub navigator: RefCell<Option<FolderNavigator>>,
        pub generation: Cell<u64>,
        pub hide_timeout: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ReelWindow {
        const NAME: &'static str = "ReelWindow";
        type Type = super::ReelWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ReelWindow {}
    impl WidgetImpl for ReelWindow {}
    impl WindowImpl for ReelWindow {}
    impl ApplicationWindowImpl for ReelWindow {}
    impl AdwApplicationWindowImpl for ReelWindow {}
}

glib::wrapper! {
    pub struct ReelWindow(ObjectSubclass<imp::ReelWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ReelWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        let window: Self = glib::Object::builder()
            .property("application", application)
            .build();

        window.setup_ui();
        window
    }

    pub fn open_path(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();

        match FolderNavigator::new(path) {
            Ok(navigator) => {
                *self.imp().navigator.borrow_mut() = Some(navigator);
                self.show_current_file();
                Ok(())
            }
            Err(error) => {
                *self.imp().navigator.borrow_mut() = None;
                self.set_title_from_path(path);
                self.show_error(&error);
                Err(error)
            }
        }
    }

    fn setup_ui(&self) {
        self.setup_open_buttons();
        self.setup_navigation();
        self.setup_actions();
    }

    fn setup_open_buttons(&self) {
        let filter = gtk::FileFilter::new();
        let filter_name = gettext("Supported Media");
        filter.set_name(Some(&filter_name));
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/webp");
        filter.add_mime_type("image/avif");
        filter.add_mime_type("image/heic");
        filter.add_mime_type("image/heif");
        filter.add_mime_type("image/jxl");
        filter.add_mime_type("image/gif");
        filter.add_mime_type("image/bmp");
        filter.add_mime_type("image/tiff");
        filter.add_mime_type("image/svg+xml");
        filter.add_mime_type("image/x-icon");
        filter.add_mime_type("image/vnd.microsoft.icon");
        filter.add_mime_type("video/mp4");
        filter.add_mime_type("video/x-matroska");
        filter.add_mime_type("video/webm");
        filter.add_mime_type("video/quicktime");
        filter.add_mime_type("video/x-msvideo");
        filter.add_mime_type("video/ogg");
        filter.add_mime_type("video/x-m4v");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title(gettext("Open Media"))
            .filters(&filters)
            .modal(true)
            .build();

        let local_files_message = gettext("Only local files are supported.");
        let dialog_error_message = gettext("Could not open the selected file.");

        self.connect_open_button(
            &self.imp().open_button,
            &dialog,
            &local_files_message,
            &dialog_error_message,
        );
        self.connect_open_button(
            &self.imp().header_open_button,
            &dialog,
            &local_files_message,
            &dialog_error_message,
        );
    }

    fn connect_open_button(
        &self,
        button: &gtk::Button,
        dialog: &gtk::FileDialog,
        local_files_message: &str,
        dialog_error_message: &str,
    ) {
        let dialog = dialog.clone();
        let local_files_message = local_files_message.to_owned();
        let dialog_error_message = dialog_error_message.to_owned();

        button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let parent = window.clone();
                let local_files_message = local_files_message.clone();
                let dialog_error_message = dialog_error_message.clone();

                dialog.open(
                    Some(&parent),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak]
                        window,
                        move |result| match result {
                            Ok(file) => {
                                if let Some(path) = file.path() {
                                    if let Err(error) = window.open_path(&path) {
                                        glib::g_warning!(
                                            "reel",
                                            "Failed to open {}: {error}",
                                            path.display()
                                        );
                                    }
                                } else {
                                    glib::g_warning!(
                                        "reel",
                                        "Only local files are supported: {}",
                                        file.uri()
                                    );
                                    window.show_toast(&local_files_message);
                                }
                            }
                            Err(error)
                                if error.matches(gtk::DialogError::Dismissed)
                                    || error.matches(gtk::DialogError::Cancelled)
                                    || error.matches(gio::IOErrorEnum::Cancelled) => {}
                            Err(error) => {
                                glib::g_warning!("reel", "File dialog failed: {error}");
                                window.show_toast(&dialog_error_message);
                            }
                        }
                    ),
                );
            }
        ));
    }

    fn setup_navigation(&self) {
        self.imp().prev_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.navigate_previous()
        ));

        self.imp().next_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.navigate_next()
        ));

        let motion_controller = gtk::EventControllerMotion::new();
        motion_controller.connect_motion(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _| window.show_controls()
        ));
        self.imp().media_overlay.add_controller(motion_controller);
    }

    fn setup_actions(&self) {
        let next_action = gio::ActionEntry::builder("next")
            .activate(|window: &Self, _, _| window.navigate_next())
            .build();
        let previous_action = gio::ActionEntry::builder("previous")
            .activate(|window: &Self, _, _| window.navigate_previous())
            .build();
        self.add_action_entries([next_action, previous_action]);
    }

    fn show_current_file(&self) {
        let path = self
            .imp()
            .navigator
            .borrow()
            .as_ref()
            .map(FolderNavigator::current_path);

        if let Some(path) = path {
            self.display_path(&path);
        }
    }

    fn current_window(weak_window: &glib::WeakRef<Self>, generation: u64) -> Option<Self> {
        let window = weak_window.upgrade()?;

        (window.imp().generation.get() == generation).then_some(window)
    }

    fn load_image(&self, path: PathBuf, generation: u64) {
        let weak_window = self.downgrade();

        glib::spawn_future_local(async move {
            let first_frame = load_first_frame(&path).await;

            let Some(window) = Self::current_window(&weak_window, generation) else {
                return;
            };

            let (mut image, frame) = match first_frame {
                Ok(result) => result,
                Err(error) => {
                    glib::g_warning!("reel", "Failed to load {}: {error}", path.display());
                    let error = anyhow::Error::new(AppError::LoadFailed(path));
                    window.show_error(&error);
                    return;
                }
            };

            window
                .imp()
                .image_view
                .set_paintable(Some(&frame.texture()));

            let Some(mut delay) = frame.delay() else {
                return;
            };

            drop(window);

            loop {
                let wait = if delay < Duration::from_millis(20) {
                    Duration::from_millis(100)
                } else {
                    delay
                };

                glib::timeout_future(wait).await;

                if Self::current_window(&weak_window, generation).is_none() {
                    return;
                }

                let frame = match image.next_frame().await {
                    Ok(frame) => frame,
                    Err(error) => {
                        if Self::current_window(&weak_window, generation).is_none() {
                            return;
                        }

                        glib::g_warning!(
                            "reel",
                            "Failed to load frame from {}: {error}",
                            path.display()
                        );
                        return;
                    }
                };

                let Some(window) = Self::current_window(&weak_window, generation) else {
                    return;
                };

                window
                    .imp()
                    .image_view
                    .set_paintable(Some(&frame.texture()));

                let Some(next_delay) = frame.delay() else {
                    return;
                };

                delay = next_delay;
                drop(window);
            }
        });
    }

    fn display_path(&self, path: &Path) {
        let generation = self.imp().generation.get().wrapping_add(1);
        self.imp().generation.set(generation);

        self.set_title_from_path(path);

        if !path.is_file() {
            let error = anyhow::Error::new(AppError::PathNotFound(path.to_path_buf()));
            self.show_error(&error);
            return;
        }

        match media_kind(path) {
            Some(MediaKind::Image) => {
                self.stop_video();
                self.imp()
                    .image_view
                    .set_paintable(None::<&gtk::gdk::Paintable>);
                self.imp().view_stack.set_visible_child_name("image");
                self.load_image(path.to_path_buf(), generation);
            }
            Some(MediaKind::Video) => {
                self.stop_video();
                self.imp()
                    .image_view
                    .set_paintable(None::<&gtk::gdk::Paintable>);
                self.imp().video_view.set_filename(Some(path));
                self.imp().view_stack.set_visible_child_name("video");
            }
            None => {
                let error = anyhow::Error::new(AppError::UnsupportedFile(path.to_path_buf()));
                self.show_error(&error);
                return;
            }
        }

        self.hide_controls();
    }

    fn navigate_next(&self) {
        if !self.can_navigate() {
            return;
        }

        let path = self
            .imp()
            .navigator
            .borrow_mut()
            .as_mut()
            .map(FolderNavigator::next);

        if let Some(path) = path {
            self.display_path(&path);
            self.show_controls();
        }
    }

    fn navigate_previous(&self) {
        if !self.can_navigate() {
            return;
        }

        let path = self
            .imp()
            .navigator
            .borrow_mut()
            .as_mut()
            .map(FolderNavigator::previous);

        if let Some(path) = path {
            self.display_path(&path);
            self.show_controls();
        }
    }

    fn stop_video(&self) {
        if let Some(stream) = self.imp().video_view.media_stream() {
            stream.pause();
        }

        self.imp().video_view.set_filename(None::<&Path>);
    }

    fn show_error(&self, error: &anyhow::Error) {
        self.imp()
            .generation
            .set(self.imp().generation.get().wrapping_add(1));

        self.stop_video();
        self.imp()
            .image_view
            .set_paintable(None::<&gtk::gdk::Paintable>);

        let message = match error.downcast_ref::<AppError>() {
            Some(app_error) => app_error.user_message(),
            None => {
                glib::g_warning!("reel", "Unexpected error: {error}");
                generic_user_message()
            }
        };

        self.imp().error_label.set_label(&message);
        self.imp().view_stack.set_visible_child_name("error");
        self.hide_controls();
    }

    fn show_toast(&self, message: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(message));
    }

    fn set_title_from_path(&self, path: &Path) {
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        self.imp().file_title.set_title(&title);
    }

    fn has_multiple_files(&self) -> bool {
        self.imp()
            .navigator
            .borrow()
            .as_ref()
            .is_some_and(FolderNavigator::has_multiple_files)
    }

    fn can_navigate(&self) -> bool {
        self.has_multiple_files()
    }

    fn show_controls(&self) {
        if !self.can_navigate() {
            return;
        }

        self.imp().prev_revealer.set_reveal_child(true);
        self.imp().next_revealer.set_reveal_child(true);

        self.cancel_hide_timeout();

        let weak = self.downgrade();
        let source = glib::timeout_add_local_once(CONTROLS_HIDE_DELAY, move || {
            if let Some(window) = weak.upgrade() {
                window.imp().hide_timeout.borrow_mut().take();
                window.hide_controls();
            }
        });

        *self.imp().hide_timeout.borrow_mut() = Some(source);
    }

    fn hide_controls(&self) {
        self.cancel_hide_timeout();
        self.imp().prev_revealer.set_reveal_child(false);
        self.imp().next_revealer.set_reveal_child(false);
    }

    fn cancel_hide_timeout(&self) {
        if let Some(source) = self.imp().hide_timeout.borrow_mut().take() {
            source.remove();
        }
    }
}
