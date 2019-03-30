use crate::notedata;
use crate::notedata::NoteData;
use std::ffi::OsStr;
use std::fs::{read_dir, File};

pub fn load_song(simfile_folder: &str) -> Option<NoteData> {
    read_dir(simfile_folder)
        .expect("Couldn't open folder")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension() == Some(OsStr::new("sm")))
        .filter_map(|sim| File::open(sim.path()).ok())
        .find_map(|sim| notedata::NoteData::from_sm(sim).ok())
}