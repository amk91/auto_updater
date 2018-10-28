use std::path::{Path, PathBuf};
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::env::current_dir;

fn main() {
	// Set environment variables
	let config_file_path: PathBuf = [
		current_dir().unwrap_or(PathBuf::new()),
		PathBuf::from("config.txt")
	].iter().collect();

	if !Path::new(&config_file_path).exists() {
		println!("Config file doesn't exist");
		std::process::exit(1);
	}

	let config_file = if let Ok(file) = File::open(&config_file_path) {
		file
	} else {
		println!("Unable to open {}", config_file_path.to_str().unwrap_or(""));
		std::process::exit(1);
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
				process_name = Some(value.to_string());
			}
		} else if line.starts_with("target_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				target_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("update_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				update_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("backup_dir=") {
			if let Some(value) = line.split('=').nth(1) {
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

	let update_dir_path = if let Some(mut value) = update_dir_path {
		if !value.ends_with("\\") {
			value.push_str("\\");
		}

		value
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

	// Check if folders and exe file exist

}
