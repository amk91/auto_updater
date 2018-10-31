use std::path::{Path, PathBuf};
use std::io::{BufReader, BufRead, ErrorKind};
use std::fs::{File, DirBuilder};
use std::env::current_dir;
use std::process::exit;

extern crate walkdir;
use walkdir::WalkDir;

extern crate zip;
use zip::ZipArchive;

fn main() {
	// Set environment variables
	let config_file_path: PathBuf = [
		current_dir().unwrap_or(PathBuf::new()),
		PathBuf::from("config.txt")
	].iter().collect();

	if !Path::new(&config_file_path).exists() {
		println!("Config file doesn't exist in {}",
			current_dir().unwrap_or(PathBuf::new()).to_str().unwrap());
		exit(1);
	}

	let config_file = if let Ok(file) = File::open(&config_file_path) {
		file
	} else {
		println!("Unable to open {}", config_file_path.to_str().unwrap_or(""));
		exit(1);
	};

	let config_file = BufReader::new(&config_file);
	let lines = config_file.lines();
	let mut process_name: Option<String> = None;
	let mut target_dir_path: Option<String> = None;
	let mut update_dir_path: Option<String> = None;
	let mut backup_dir_path: Option<String> = None;
	for line in lines.filter(|x| x.is_ok()) {
		let line = line.unwrap();
		if line.starts_with("process=") {
			if let Some(value) = line.split('=').nth(1) {
				if !&value.ends_with(".exe") {
					panic!("The name {} provided for the process is not a valid one", value);
				}

				process_name = Some(value.to_string());
			}
		} else if line.starts_with("target_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					panic!("The path {} does not exist", value);
				}

				target_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("update_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					panic!("The path {} does not exist", value);
				}

				update_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("backup_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					panic!("The path {} does not exist", value);
				}

				backup_dir_path = Some(value.to_string());
			}
		}

		if process_name.is_some() && target_dir_path.is_some()
		&& update_dir_path.is_some() && backup_dir_path.is_some() {
			break;
		}
	}

	let process_name = if let Some(value) = process_name {
		value
	} else {
		panic!("No process name given");
	};

	let target_dir_path = if let Some(mut value) = target_dir_path {
		if !value.ends_with("\\") {
			value.push_str("\\")
		}

		value
	} else {
		panic!("No target dir given");
	};

	let (update_history_dir_path, update_dir_path) = if let Some(mut value) = update_dir_path {
		if !value.ends_with("\\") {
			value.push_str("\\");
		}

		let auto_updater_dir_path = format!("{}__auto_updater\\", value);
		if let Err(err) = DirBuilder::new().create(&auto_updater_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				panic!("Error creating folder, {}", err);
			}
		}

		let auto_updater_history_dir_path = format!("{}__auto_updater_history\\", value);
		if let Err(err) = DirBuilder::new().create(&auto_updater_history_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				panic!("Error creating folder, {}", err);
			}
		}

		(auto_updater_history_dir_path, auto_updater_dir_path)
	} else {
		panic!("No update dir given");
	};

	let backup_dir_path = if let Some(mut value) = backup_dir_path {
		if !value.ends_with("\\") {
			value.push_str("\\");
		}

		value
	} else {
		panic!("No backup dir given");
	};

	'main: loop {
		for entry in WalkDir::new(&update_dir_path).into_iter().filter_map(|e| e.ok()) {
			if let Some(extension) = entry.path().extension() {
				let archive_file_path = if extension == "zip" {
					entry.path().to_str().unwrap_or("")
				} else {
					continue;
				};

				let mut archive = if let Ok(archive_file) = File::open(&archive_file_path) {
					if let Ok(archive) = ZipArchive::new(archive_file) {
						archive
					} else {
						println!("Unable to open zip file {}", archive_file_path);

						//TODO: move the file into a different folder

						continue;
					}
				} else {
					println!("Unable to open file {}", archive_file_path);

					//TODO: move the file into a different folder

					continue;
				};

				for i in 0..archive.len() {
					let mut file = if let Ok(file) = archive.by_index(i) {
						file
					} else {
						println!("Unable to open file inside the archive");
						break;
					};
				}
			}
		}
	}
}
