#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use std::io::{self, Write, BufReader, BufRead, ErrorKind};
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

#[cfg(windows)] extern crate winapi;

fn message_box(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{MB_OK, MB_SYSTEMMODAL, MB_ICONWARNING, MessageBoxW};

    let message: Vec<u16> = OsStr::new(message)
    	.encode_wide()
    	.chain(once(0))
    	.collect();
    let title: Vec<u16> = OsStr::new(title)
    	.encode_wide()
    	.chain(once(0))
    	.collect();
    unsafe {
        MessageBoxW(
        	null_mut(),
        	message.as_ptr(),
        	title.as_ptr(),
        	MB_OK | MB_SYSTEMMODAL | MB_ICONWARNING
        );
    }
}

enum ErrorType {
	Critical,
	Warning,
}

fn log_error(error_file: &mut File, message: &str, error_type: ErrorType) {
	let date = Local::now();
	let error_type = match error_type {
		ErrorType::Critical => 'E',
		ErrorType::Warning => 'W',
	};
	let message = format!(
		"{}/{}/{} {}:{}:{} {}: {}",
		date.year(), date.month(), date.day(),
		date.hour(), date.minute(), date.second(),
		error_type, message
	);

	error_file.write_all(message.as_bytes()).unwrap_or(());
}

fn main() {
	let error_file_path: PathBuf = [
		current_dir().unwrap_or_default(),
		PathBuf::from("error_log_auto_updater.txt")
	].iter().collect();

	let mut error_file = if let Ok(file) = File::create(error_file_path) {
		file
	} else {
		message_box("Error", "Unable to create error log file");
		exit(1);
	};

	// Set environment variables
	let config_file_path: PathBuf = [
		current_dir().unwrap_or_default(),
		PathBuf::from("config.txt")
	].iter().collect();

	if !Path::new(&config_file_path).exists() {
		log_error(
			&mut error_file,
			&format!(
				"Config file doesn't exist in {}",
				current_dir().unwrap_or_default().to_str().unwrap()
			),
			ErrorType::Critical
		);
		exit(1);
	}

	let config_file = if let Ok(file) = File::open(&config_file_path) {
		file
	} else {
		log_error(
			&mut error_file,
			&format!(
				"Unable to open {}",
				config_file_path.to_str().unwrap_or("")
			),
			ErrorType::Critical
		);
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
					log_error(
						&mut error_file,
						&format!(
							"The name {} provided for the process is not a valid one",
							value
						),
						ErrorType::Critical
					);
					exit(1);
				}

				process_name = Some(value.to_string());
			}
		} else if line.starts_with("target_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					log_error(
						&mut error_file,
						&format!("The path {} does not exist", value),
						ErrorType::Critical
					);
					exit(1);
				}

				target_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("update_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					log_error(
						&mut error_file,
						&format!("The path {} does not exist", value),
						ErrorType::Critical
					);
					exit(1);
				}

				update_dir_path = Some(value.to_string());
			}
		} else if line.starts_with("backup_dir=") {
			if let Some(value) = line.split('=').nth(1) {
				if !Path::new(&value).exists() {
					log_error(
						&mut error_file,
						&format!("The path {} does not exist", value),
						ErrorType::Critical
					);
					exit(1);
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
		log_error(&mut error_file, "No process name given", ErrorType::Critical);
		exit(1);
	};

	let target_dir_path = if let Some(mut value) = target_dir_path {
		if !value.ends_with('\\') {
			value.push_str("\\")
		}

		value
	} else {
		log_error(
			&mut error_file,
			"No target directory path given",
			ErrorType::Critical
		);
		exit(1);
	};

	let (update_history_dir_path, update_dir_path) = if let Some(mut value) = update_dir_path {
		if !value.ends_with('\\') {
			value.push_str("\\");
		}

		let auto_updater_dir_path = format!("{}__auto_updater\\", value);
		if let Err(err) = DirBuilder::new().create(&auto_updater_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				log_error(
					&mut error_file,
					&format!("Unable to create folder - {}", err),
					ErrorType::Critical
				);
				exit(1);
			}
		}

		let auto_updater_history_dir_path = format!("{}__auto_updater_history\\", value);
		if let Err(err) = DirBuilder::new().create(&auto_updater_history_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				log_error(
					&mut error_file,
					&format!("Unable to create folder - {}", err),
					ErrorType::Critical,
				);
				exit(1);
			}
		}

		(auto_updater_history_dir_path, auto_updater_dir_path)
	} else {
		log_error(
			&mut error_file,
			"No update directory path given",
			ErrorType::Critical
		);
		exit(1);
	};

	let (backup_dir_path, error_backup_dir_path) = if let Some(mut value) = backup_dir_path {
		if !value.ends_with('\\') {
			value.push_str("\\");
		}

		let error_backup_dir_path = format!("{}{}", value, "__auto_updater_error\\");
		if let Err(err) = DirBuilder::new().create(&error_backup_dir_path) {
			if err.kind() != ErrorKind::AlreadyExists {
				log_error(
					&mut error_file,
					&format!("Unable to create folder {}", error_backup_dir_path),
					ErrorType::Critical
				);
				exit(1);
			}
		}

		let backup_dir_path = value;

		(backup_dir_path, error_backup_dir_path)
	} else {
		log_error(
			&mut error_file,
			&format!("No backup directory path given"),
			ErrorType::Critical
		);
		exit(1);
	};

	'main: loop {
		let mut update_completed = false;
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
						log_error(
							&mut error_file,
							&format!(
								"Unable to open zip file {}",
								archive_file_path.to_str().unwrap_or("")
							),
							ErrorType::Warning
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
							log_error(
								&mut error_file,
								&format!(
									"Unable to move the file {}",
									&archive_file_path.to_str().unwrap_or("")
								),
								ErrorType::Warning
							);
						}

						continue;
					}
				} else {
					log_error(
						&mut error_file,
						&format!(
							"Unable to open file {}",
							archive_file_path.to_str().unwrap_or("")
						),
						ErrorType::Warning
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
						log_error(
							&mut error_file,
							&format!(
								"Unable to move the file {}",
								archive_file_path.to_str().unwrap_or("")
							),
							ErrorType::Warning
						);
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
						message_box(
							"Update available",
							"A new update is ready\nPlease close the software and wait for the update to be finished\nPress OK to continue"
						);
					}

					thread::sleep(Duration::from_millis(1000));
				}

				let update_backup_dir_path = format!(
					"{}\\{}-{}-{}-{}-{}-{}\\",
					backup_dir_path,
					date.year(), date.month(), date.day(),
					date.hour(), date.minute(), date.second()
				);

				if let Err(err) = DirBuilder::new().create(&update_backup_dir_path) {
					if err.kind() != ErrorKind::AlreadyExists {
						log_error(
							&mut error_file,
							&format!(
								"Unable to create folder {}",
								update_backup_dir_path
							),
							ErrorType::Warning
						);
						break;
					}
				}

				for i in 0..archive.len() {
					let mut item = if let Ok(item) = archive.by_index(i) {
						item
					} else {
						log_error(
							&mut error_file,
							&format!("Unable to open item inside the archive"),
							ErrorType::Warning
						);
						break;
					};

					if item.name().ends_with('/') {
						if let Some(item_directory_name) = item.sanitized_name().to_str() {
							let item_backup_dir_path = format!(
								"{}{}",
								update_backup_dir_path,
								item_directory_name
							);

							let item_target_dir_path = format!(
								"{}{}",
								target_dir_path,
								item_directory_name
							);

							if let Err(err) = DirBuilder::new().create(
								&item_target_dir_path
							) {
								if err.kind() != ErrorKind::AlreadyExists {
									log_error(
										&mut error_file,
										&format!(
											"Unable to create folder
											in target directory {}",
											item_target_dir_path
										),
										ErrorType::Warning
									);
								} else {
									if let Err(err) = DirBuilder::new().create(
										&item_backup_dir_path
									) {
										if err.kind() != ErrorKind::AlreadyExists {
											log_error(
												&mut error_file,
												&format!(
													"Unable to create folder 
													in backup directory {}",
													item_backup_dir_path
												),
												ErrorType::Warning	
											);
										}
									}
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
								log_error(
									&mut error_file,
									&format!(
										"Unable to move the file {} 
										inside the backup folder",
										update_backup_dir_path
									),
									ErrorType::Warning
								);
								break;
							}
						} else {
							log_error(
								&mut error_file,
								&format!(
									"File {} does not exist",
									current_file_path
								),
								ErrorType::Warning
							);
						}

						if let Ok(mut current_file) = File::create(&current_file_path) {
							if let Err(_) = io::copy(&mut item, &mut current_file) {
								log_error(
									&mut error_file,
									&format!(
										"Unable to transfer file from archive to {}",
										current_file_path
									),
									ErrorType::Warning
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
								log_error(
									&mut error_file,
									&format!(
										"Unable to create folder {}",
										update_history_archive_folder_path
									),
									ErrorType::Warning
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
								log_error(
									&mut error_file,
									&format!(
										"Unable to move archive {} - {}",
										archive_file_path.to_str().unwrap_or(""),
										err
									),
									ErrorType::Warning
								);
							}
						}
					}
				}

				update_completed = true;
			}
		}

		if update_completed {
			message_box(
				"Update completed",
				"Software has been successfully updated\nYou can start the software now"
			);
		}
		thread::sleep(Duration::from_millis(30_000));
	}
}
