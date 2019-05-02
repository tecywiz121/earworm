#[macro_use]
pub extern crate slog;

mod game;
mod media;

use glib::functions::set_application_name;
use glib::variant::FromVariant;

use gio::prelude::*;

use gtk::prelude::*;
use gtk::{AboutDialog, ToVariant};

use std::env::args;
use std::path::Path;

#[derive(Debug, Clone)]
struct QuizButton {
    button: gtk::Button,
    image: gtk::Image,
    label: gtk::Label,
}

impl QuizButton {
    pub fn new() -> Self {
        let button = gtk::Button::new();

        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        button.add(&v_box);

        let image = gtk::Image::new();
        v_box.pack_start(&image, true, true, 0);

        let label = gtk::Label::new("");
        v_box.pack_start(&label, false, false, 0);

        Self {
            button,
            image,
            label,
        }
    }
}

#[derive(Debug, Clone)]
struct Earworm {
    application: gtk::Application,
    window: gtk::ApplicationWindow,

    remaining: gtk::ProgressBar,

    first: QuizButton,
    second: QuizButton,
    third: QuizButton,
}

impl Earworm {
    pub fn new(application: gtk::Application) -> Self {
        let window = gtk::ApplicationWindow::new(&application);

        window.set_title("Earworm");
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 70);

        let toolbar = gtk::Toolbar::new();

        let image_open =
            gtk::Image::new_from_icon_name("document-open", gtk::IconSize::SmallToolbar);
        let tool_open = gtk::ToolButton::new(&image_open, "Open Folder...");
        tool_open.set_action_name("app.open");

        toolbar.insert(&tool_open, 0);

        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);

        let remaining = gtk::ProgressBar::new();

        v_box.pack_start(&toolbar, false, false, 0);
        v_box.pack_start(&remaining, false, false, 0);
        window.add(&v_box);

        let f_box = gtk::FlowBox::new();
        f_box.set_halign(gtk::Align::Center);
        f_box.set_valign(gtk::Align::Center);
        f_box.set_orientation(gtk::Orientation::Vertical);
        f_box.set_column_spacing(10);
        f_box.set_row_spacing(10);

        v_box.pack_start(&f_box, true, true, 0);

        let first = QuizButton::new();
        f_box.add(&first.button);

        let second = QuizButton::new();
        f_box.add(&second.button);

        let third = QuizButton::new();
        f_box.add(&third.button);

        Self::build_system_menu(&application);

        let result = Earworm {
            window,
            application,
            remaining,

            first,
            second,
            third,
        };

        result.add_actions();
        result
    }

    pub fn show_all(&self) {
        self.window.show_all();
    }

    fn build_system_menu(application: &gtk::Application) {
        let menu = gio::Menu::new();
        menu.append("About Earworm", "app.about");
        application.set_app_menu(&menu);
    }

    fn add_actions(&self) {
        let about = gio::SimpleAction::new("about", None);

        let win = self.window.clone();
        about.connect_activate(move |_, _| {
            let p = AboutDialog::new();
            p.set_authors(&["Sam Wilson"]);
            p.set_website(Some("https://github.com/tecywiz121/earworm"));
            p.set_title("About Earworm");
            p.set_transient_for(Some(&win));
            p.run();
            p.destroy();
        });

        self.application.add_action(&about);

        let start_p = glib::VariantTy::new("s").unwrap();
        let start = gio::SimpleAction::new("start", start_p);
        start.connect_activate(move |_, p| {
            let p = p.as_ref().expect("app.start activated without parameter");

            // TODO: Paths aren't technically strings.
            let p_string = String::from_variant(p).expect("app.start activated with non-string");
            let path = Path::new(&p_string);

            println!("{}", path.to_string_lossy());
        });

        self.application.add_action(&start);

        let open = gio::SimpleAction::new("open", None);
        let win = self.window.clone();
        let app = self.application.clone();
        open.connect_activate(move |_, _| {
            let p = gtk::FileChooserDialog::with_buttons(
                "Choose Music Folder",
                Some(&win),
                gtk::FileChooserAction::SelectFolder,
                &[
                    ("Cancel", gtk::ResponseType::Cancel),
                    ("Select", gtk::ResponseType::Accept),
                ],
            );

            if p.run() == gtk::ResponseType::Accept.into() {
                if let Some(dirname) = p.get_filename() {
                    let action = app.lookup_action("start").unwrap();
                    let path = dirname.to_str().expect("paths must be UTF-8");

                    action.activate(&path.to_variant());
                }
            }

            p.destroy();
        });

        self.application.add_action(&open);
    }
}

fn main() {
    set_application_name("Earworm");

    let application = gtk::Application::new("com.github.tecywiz121.earworm", Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        let ew = Earworm::new(app.clone());
        ew.show_all();
    });

    application.run(&args().collect::<Vec<_>>());
}
