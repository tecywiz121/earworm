use id3::frame::PictureType;
use id3::Tag;

use slog::Logger;

use snafu::{OptionExt, ResultExt, Snafu};

use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    NoMetadata,
}

#[derive(Debug, Snafu)]
enum TagError {
    Id3 { source: id3::Error },
    NoTitle,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Image {
    mime_type: String,
    data: Vec<u8>,
}

impl Image {
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Track {
    path: PathBuf,

    artist: Option<String>,
    album: Option<String>,
    title: String,

    cover: Option<Image>,
}

impl Track {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn artist(&self) -> Option<&str> {
        self.artist.as_ref().map(String::as_str)
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_ref().map(String::as_str)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn cover(&self) -> Option<&Image> {
        self.cover.as_ref()
    }

    fn title_from(path: &Path) -> Option<String> {
        match path.file_stem() {
            Some(t) => Some(t.to_string_lossy().into_owned()),
            _ => None,
        }
    }

    fn find_cover(tag: &Tag) -> Option<Image> {
        let mut best = None;

        // TODO: Come up with a better ordering for the picture types
        for pic in tag.pictures() {
            best = Some(pic);
            if pic.picture_type == PictureType::CoverFront {
                break;
            }
        }

        best.map(|x| Image {
            mime_type: x.mime_type.clone(),
            data: x.data.clone(),
        })
    }

    fn try_from_file(path: PathBuf, logger: &Logger) -> Result<Self, TagError> {
        let tag = Tag::read_from_path(&path).context(Id3)?;

        let title = match tag.title() {
            Some(x) => x.to_owned(),
            None => Self::title_from(&path).context(NoTitle)?,
        };

        Ok(Track {
            cover: Self::find_cover(&tag),
            artist: tag.artist().map(ToOwned::to_owned),
            album: tag.album().map(ToOwned::to_owned),
            title,
            path,
        })
    }

    pub fn from_file<P: Into<PathBuf>>(file: P, logger: &Logger) -> Result<Self, Error> {
        let path = file.into();

        let err = match Self::try_from_file(path.clone(), logger) {
            Err(e) => e,
            Ok(track) => return Ok(track),
        };

        debug!(logger, "error while reading metadata: {:#?}", err,);

        warn!(
            logger,
            "unable to read any metadata from `{}`, using filename instead",
            path.to_string_lossy(),
        );

        Ok(Track {
            cover: None,
            artist: None,
            album: None,
            title: Self::title_from(&path).context(NoMetadata)?,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_file_full_id3() {
        let logger = slog::Logger::root(slog::Discard, o!());

        let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "full-id3.mp3"]
            .iter()
            .collect();

        let track = Track::from_file(&path, &logger).unwrap();

        assert_eq!(track.path, path);
        assert_eq!(track.artist, Some("Santa Clause".into()));
        assert_eq!(track.album, Some("The Red Album".into()));
        assert_eq!(track.title, "Red Square");

        let image = track.cover.expect("should have a cover image");
        assert_eq!(image.mime_type, "image/png");
    }
}
