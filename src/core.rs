use crate::units::{Unit, ALL_UNITS};
use std::ffi::OsStr;
use std::path::Path;
use std::process;

pub fn initialize(path: &str) {
    load_all_services(path);
}

pub fn load_all_services(path: &str) {
    let services_path = Path::new(path);

    if !services_path.exists() {
        println!("Give {} path does not exist...", services_path.display());
        process::exit(1);
    }
    if !services_path.is_dir() {
        println!("Expected {} to a directory...", services_path.display());
        process::exit(1);
    }

    let all_services = services_path
        .read_dir()
        .expect("read_dir call failed")
        .filter(|entry| {
            entry
                .as_ref()
                .expect("Failed to check if the path is a file")
                .path()
                .is_file()
        })
        .filter(|entry| {
            entry
                .as_ref()
                .expect("Failed to check if the path has service extension")
                .path()
                .extension()
                == Some(OsStr::new("service"))
        });

    for entry in all_services {
        if let Ok(an_entry) = entry {
            println!("Loading {:?}...", an_entry);

            let unit = Unit::from_unitfile(&an_entry.path().as_path());
            ALL_UNITS.lock().expect("Failed to parse unit file").add_unit(unit);
        }
    }
}
