use crate::media::Track;

use slog::Logger;

use snafu::Snafu;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Snafu)]
pub enum Error {}

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

    pub fn search_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<(), Error> {
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

        Ok(())
    }
}
