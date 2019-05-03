use crate::media::Track;

use rand::seq::IteratorRandom;
use rand::Rng;

use slog::Logger;

use snafu::Snafu;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub struct Game {
    logger: Logger,
    tracks: HashSet<Track>,
}

impl Game {
    pub fn new(logger: Logger) -> Game {
        Game {
            logger,
            tracks: Default::default(),
        }
    }

    pub fn search_dir<P: AsRef<Path>>(&mut self, dir: P) {
        let dir = dir.as_ref();

        let dir_str = dir.to_string_lossy().into_owned();
        let logger = self.logger.new(o!("search-dir" => dir_str));

        let tracks = WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|x| x.ok())
            .filter(|x| match x.metadata() {
                Ok(md) => md.is_file(),
                _ => false,
            })
            .filter(|x| x.path().extension() == Some(OsStr::new("mp3")))
            .filter_map(|x| Track::from_file(x.into_path(), &logger).ok());

        self.tracks.extend(tracks);
    }

    pub fn start_round(&self, options: usize) -> Round {
        let mut rng = rand::thread_rng();
        let tracks: Vec<_> = self
            .tracks
            .iter()
            .choose_multiple(&mut rng, options)
            .into_iter()
            .map(|x| (*x).clone())
            .collect();

        Round {
            correct_idx: rng.gen_range(0, tracks.len()),
            ends: Instant::now() + Duration::from_secs(10),
            tracks,
        }
    }
}

#[derive(Debug)]
pub struct Round {
    correct_idx: usize,
    tracks: Vec<Track>,

    ends: Instant,
}

impl Round {
    pub fn ends(&self) -> Instant {
        self.ends
    }

    pub fn correct(&self) -> &Track {
        &self.tracks[self.correct_idx]
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }
}
