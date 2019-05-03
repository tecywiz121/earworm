#[macro_use]
pub extern crate slog;

mod game;
mod media;

use crate::game::{Game, Round};
use crate::media::Track;

use derivative::Derivative;

use glib::functions::set_application_name;
use glib::variant::FromVariant;

use gio::prelude::*;

use gtk::prelude::*;
use gtk::{AboutDialog, ToVariant};

use rodio::{Sink, Source};

use slog::{Drain, Logger};

use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct QuizButton {
    button: gtk::Button,
    image: gtk::Image,

    artist: gtk::Label,
    album: gtk::Label,
    title: gtk::Label,
}

impl QuizButton {
    pub fn new() -> Self {
        let button = gtk::Button::new();

        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        button.add(&v_box);

        let image = gtk::Image::new();
        v_box.pack_start(&image, true, true, 0);

        let artist = gtk::Label::new("");
        v_box.pack_start(&artist, false, false, 0);

        let album = gtk::Label::new("");
        v_box.pack_start(&album, false, false, 0);

        let title = gtk::Label::new("");
        v_box.pack_start(&title, false, false, 0);

        Self {
            button,
            image,
            artist,
            album,
            title,
        }
    }

    pub fn set_album(&self, txt: &str) {
        self.album.set_text(txt);
    }

    pub fn set_artist(&self, txt: &str) {
        self.artist.set_text(txt);
    }

    pub fn set_title(&self, txt: &str) {
        self.title.set_text(txt);
    }

    pub fn set_from_track(&self, track: &Track) {
        if let Some(album) = track.album() {
            self.set_album(album);
        }

        if let Some(artist) = track.artist() {
            self.set_artist(artist);
        }

        self.set_title(track.title());
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
struct Earworm {
    application: gtk::Application,
    window: gtk::ApplicationWindow,

    remaining: gtk::ProgressBar,

    first: QuizButton,
    second: QuizButton,
    third: QuizButton,

    #[derivative(Debug = "ignore")]
    sink: Arc<Mutex<Sink>>,

    round: Arc<Mutex<Option<Round>>>,
    game: Arc<Mutex<Game>>,
}

impl Earworm {
    pub fn new(application: gtk::Application, logger: Logger) -> Self {
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

        let image_play =
            gtk::Image::new_from_icon_name("media-playback-start", gtk::IconSize::SmallToolbar);
        let tool_play = gtk::ToolButton::new(&image_play, "Start Game");
        tool_play.set_action_name("app.play");

        toolbar.insert(&tool_play, 1);

        let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);

        let remaining = gtk::ProgressBar::new();
        remaining.set_inverted(true);

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

        let device = rodio::default_output_device().unwrap();
        let sink = Arc::new(Mutex::new(Sink::new(&device)));

        let result = Earworm {
            window,
            application,
            remaining,

            first,
            second,
            third,

            sink,
            round: Arc::default(),
            game: Arc::new(Mutex::new(Game::new(logger))),
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
        let game = self.game.clone();
        start.connect_activate(move |_, p| {
            let p = p.as_ref().expect("app.start activated without parameter");

            // TODO: Paths aren't technically strings.
            let p_string = String::from_variant(p).expect("app.start activated with non-string");
            let path = Path::new(&p_string);

            game.lock().unwrap().search_dir(path);
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

        let play = gio::SimpleAction::new("play", None);
        let ear = self.clone();
        play.connect_activate(move |_, _| {
            let game = ear.game.lock().unwrap();

            let round = game.start_round(3);

            let full_duration = round.ends() - Instant::now();
            let tick_duration = (full_duration) / 500;

            let tracks = round.tracks();
            ear.first.set_from_track(&tracks[0]);
            ear.second.set_from_track(&tracks[1]);
            ear.third.set_from_track(&tracks[2]);

            let file = File::open(round.correct().path()).unwrap();
            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
            let source = source.take_duration(full_duration);

            let device = rodio::default_output_device().unwrap();
            let mut sink = ear.sink.lock().unwrap();
            sink.stop();
            *sink = Sink::new(&device);
            sink.append(source);

            if ear.round.lock().unwrap().replace(round).is_none() {
                let ear_clone = ear.clone();
                gtk::timeout_add(tick_duration.as_millis() as u32, move || {
                    let mut guard = ear_clone.round.lock().unwrap();

                    let round = match &mut *guard {
                        Some(r) => r,
                        None => return gtk::Continue(false),
                    };

                    let now = Instant::now();

                    if now >= round.ends() {
                        std::mem::drop(round);

                        *guard = None;
                        return gtk::Continue(false);
                    }

                    let so_far = (round.ends() - now).as_millis() as f64;
                    let full = full_duration.as_millis() as f64;

                    let percent = so_far / full;

                    ear_clone.remaining.set_fraction(percent);

                    gtk::Continue(true)
                });
            }
        });

        self.application.add_action(&play);
    }
}

fn new_root_logger() -> slog::Logger {
    let decorator = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();

    slog::Logger::root(drain, o!())
}

fn main() {
    set_application_name("Earworm");

    let application = gtk::Application::new("com.github.tecywiz121.earworm", Default::default())
        .expect("Initialization failed...");

    application.connect_activate(|app| {
        let logger = new_root_logger();
        let ew = Earworm::new(app.clone(), logger);
        ew.show_all();
    });

    application.run(&args().collect::<Vec<_>>());
}
