use std::{collections::hash_map::Entry, env::consts::OS, fs::{read_to_string, File, OpenOptions}, io::{Write, Error as IoError}, path::PathBuf};
use dirs::{document_dir, home_dir};
use rustc_hash::FxHashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackerError {
    #[error("No se pudo obtener la carpeta de guardado.")]
    SavePath,

    #[error("Hubo un error al crear, escribir o leer el archivo de guardado: {0}.")]
    SaveFile(#[from] IoError),

    #[error("El sistema operativo no esta soportado.")]
    UnsupportedOs
}

pub struct EpisodeTracker {
    episodes: FxHashMap<String, Vec<i32>>,
    path: PathBuf
}

impl EpisodeTracker {
    pub fn new() -> Result<Self, TrackerError> {
        let path = match OS {
            "windows" => {
                document_dir()
                    .map(|res| res.join(".quanires.watched"))
                    .ok_or(TrackerError::SavePath)
            },
            "linux" => {
                home_dir()
                    .map(|res| res.join(".quanires.watched"))
                    .ok_or(TrackerError::SavePath)
            },
            _ => return Err(TrackerError::UnsupportedOs)
        }?;

        if !path.exists() {
            File::create(&path)?;
        }

        Ok(Self {
            episodes: read_to_string(&path)?
                .split('\n')
                .flat_map(|anime| anime
                    .split_once(" <> ")
                    .map(|split| (
                        split.0
                            .to_string(),
                        split.1
                            .split(',')
                            .flat_map(|ep| ep.parse())
                            .collect()
                    ))
                )
                .collect(),
            path
        })
    }

    pub fn episode_is_seen(&self, url: &str, episode: &i32) -> bool {
        self.episodes
            .iter()
            .any(|anime| anime.0 == url && anime.1.contains(episode))
    }

    fn save_state(&self) -> Result<(), TrackerError> {
        let state = self.episodes
            .iter()
            .map(|anime| format!(
                "{} <> {}",
                anime.0,
                anime.1
                    .iter()
                    .map(|episode| episode.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ))
            .collect::<Vec<String>>()
            .join("\n");

        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?
            .write_all(state.as_bytes())?;

        Ok(())
    }

    pub fn watch_episode(&mut self, url: &str, episode: i32) -> Result<(), TrackerError> {
        if self.episode_is_seen(url, &episode) {
            return Ok(());
        }

        if let Entry::Vacant(e) = self.episodes.entry(url.to_owned()) {
            e.insert(vec![episode]);
        } else {
            self.episodes.get_mut(url)
                .map_or_else(|| (), |s| s.push(episode));
        }

        self.save_state()
    }

    pub fn unwatch_episode(&mut self, url: &str, episode: i32) -> Result<(), TrackerError> {
        if !self.episode_is_seen(url, &episode) {
            return Ok(());
        }

        if self.episodes.contains_key(url) {
            self.episodes.get_mut(url)
                .map_or_else(|| (), |s| { s.retain(|&e| e != episode); });
        };

        self.save_state()
    }
}
