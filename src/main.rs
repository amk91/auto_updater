use std::path::{Path, PathBuf};
use std::io::{self, BufReader, BufRead, ErrorKind};
use std::fs::{self, File, DirBuilder};
use std::env::current_dir;
use std::process::{Command, exit};
use std::time::Duration;
use std::thread;

extern crate walkdir;
use walkdir::WalkDir;

extern crate zip;
use zip::ZipArchive;

extern crate chrono;
use chrono::offset::Local;
use chrono::{Timelike, Datelike};

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
				panic!("Unable to create folder, {}", err);
			}
		}

		let auto_updater_history_dir_path = format!("{}__auto_updater_history\\", value);
		if let Err(err) = DirBuilder::new().create(&auto_updater_history_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				panic!("Unable to create folder, {}", err);
			}
		}

		(auto_updater_history_dir_path, auto_updater_dir_path)
	} else {
		panic!("No update dir given");
	};

	let (error_backup_dir_path, backup_dir_path) = if let Some(mut value) = backup_dir_path {
		if !value.ends_with("\\") {
			value.push_str("\\");
		}

		(format!("{}{}", value, "__auto_updater_error\\"), value)		
	} else {
		panic!("No backup dir given");
	};

	'main: loop {
		for entry in WalkDir::new(&update_dir_path)
		.into_iter()
		.filter_map(|e| e.ok()) {
			if let Some(extension) = entry.path().extension() {
				let archive_file_path = if extension == "zip" {
					entry.path()
				} else {
					continue;
				};

				let date = Local::now();
				let error_backup_folder_path = format!(
					"{}\\{}-{}-{}-{}-{}-{}\\",
					error_backup_dir_path,
					date.year(), date.month(), date.day(),
					date.hour(), date.minute(), date.second()
				);

				let mut archive = if let Ok(archive_file) = File::open(&archive_file_path) {
					if let Ok(archive) = ZipArchive::new(archive_file) {
						archive
					} else {
						println!(
							"Unable to open zip file {}",
							archive_file_path.to_str().unwrap_or("")
						);

						if let Some(file_name) = archive_file_path.file_name() {
							fs::rename(
								&archive_file_path,
								&format!(
									"{}{}",
									error_backup_folder_path,
									file_name.to_str().unwrap_or("")
								)
							).unwrap_or(());
						} else {
							println!("|-> Unable to move the file");
						}

						continue;
					}
				} else {
					println!(
						"Unable to open file {}",
						archive_file_path.to_str().unwrap_or("")
					);

					if let Some(file_name) = archive_file_path.file_name() {
						fs::rename(
							&archive_file_path,
							&format!(
								"{}{}",
								error_backup_folder_path,
								file_name.to_str().unwrap_or("")
							)
						).unwrap_or(());
					} else {
						println!("|-> Unable to move the file");
					}

					continue;
				};

				let mut is_update_waiting_for_process = false;
				'wait_for_process: loop {
					let output = Command::new("tasklist")
						.output()
						.expect("Failed to execute command 'tasklist'");
					let output = String::from_utf8_lossy(&output.stdout);
					if !output.contains(&process_name) {
						break 'wait_for_process;
					} else if !is_update_waiting_for_process {
						is_update_waiting_for_process = true;
						print!(
							"Waiting for process {} to be closed to perform the update",
							process_name
						);
					} else {
						print!(".");
					}

					thread::sleep(Duration::from_millis(1000));
				}

				// let process_path = format!(
				// 	"{}\\{}",
				// 	target_dir_path,
				// 	process_name
				// );

				// if let Err(_) = fs::rename(
				// 	&process_path,
				// 	format!("{}{}", process_path, "__TMP")
				// ) {
				// 	println!("Unable to rename {}", process_name);
				// 	continue;
				// }

				let update_backup_dir_path = format!(
					"{}\\{}-{}-{}-{}-{}-{}\\",
					backup_dir_path,
					date.year(), date.month(), date.day(),
					date.hour(), date.minute(), date.second()
				);

				if let Err(err) = DirBuilder::new().create(&update_backup_dir_path) {
					if err.kind() != ErrorKind::AlreadyExists {
						println!(
							"Unable to create folder {}",
							update_backup_dir_path
						);
						break;
					}
				}

				for i in 0..archive.len() {
					let mut item = if let Ok(item) = archive.by_index(i) {
						item
					} else {
						println!("Unable to open item inside the archive");
						break;
					};

					if item.name().ends_with('/') {
						if let Some(directory_name) = item.sanitized_name().to_str() {
							let backup_directory_path = format!(
								"{}{}",
								update_backup_dir_path,
								directory_name
							);

							if let Err(err) = fs::create_dir(&backup_directory_path) {
								if err.kind() != ErrorKind::AlreadyExists {
									println!(
										"Unable to create folder in backup directory {}",
										update_backup_dir_path
									);
									break;
								}
							}
						}
					} else {
						let item_local_path = item.name().replace("/", "\\");
						let backup_file_path = format!(
							"{}{}",
							update_backup_dir_path,
							item_local_path
						);

						let current_file_path = format!(
							"{}{}",
							target_dir_path,
							item_local_path
						);

						if Path::new(&current_file_path).exists() {
							if let Err(_) = fs::rename(
								&current_file_path,
								backup_file_path
							) {
								println!(
									"Unable to move the file {} inside the backup folder {}",
									current_file_path,
									update_backup_dir_path
								);
								break;
							}
						} else {
							println!(
								"File {} does not exist",
								current_file_path
							);
						}

						if let Ok(mut current_file) = File::create(&current_file_path) {
							if let Err(_) = io::copy(&mut item, &mut current_file) {
								println!(
									"Unable to transfer file from archive to {}",
									current_file_path
								);
								break;
							}
						}
					}
				}

				//archive_file_path
				if let Some(archive_file_name) = archive_file_path.file_name() {
					if let Some(archive_file_name) = archive_file_name.to_str() {
						let update_history_archive_folder_path = format!(
							"{}{}-{}-{}-{}-{}-{}\\",
							update_history_dir_path,
							date.year(), date.month(), date.day(),
							date.hour(), date.minute(), date.second()
						);

						if let Err(err) = DirBuilder::new().create(
							&update_history_archive_folder_path
						) {
							if err.kind() != ErrorKind::AlreadyExists {
								println!(
									"Unable to create folder {}",
									update_history_archive_folder_path
								);
							}
						} else {
							let update_history_archive_path = format!(
								"{}{}",
								update_history_archive_folder_path,
								archive_file_name
							);
							if let Err(err) = fs::rename(
								&archive_file_path,
								&update_history_archive_path
							) {
								println!(
									"Unable to move archive {}",
									archive_file_path.to_str().unwrap_or("")
								);
								println!("|->{}", err);
							}
						}
					}
				}

				// After update is done
				// if let Err(_) = fs::rename(
				// 	format!("{}{}", process_name, "__TMP"),
				// 	&process_name
				// ) {
				// 	println!("Unable to rename back {}", process_name);
				// 	continue;
				// }
			}
		}

		println!("End cycle");
		thread::sleep(Duration::from_millis(30_000));
	}
}
