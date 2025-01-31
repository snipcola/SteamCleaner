use lazy_static::lazy_static;
use dont_disappear::any_key_to_continue;
use colored::{Colorize, control::set_virtual_terminal};
use sysinfo::System;
use privilege::user;
use winreg::{RegKey, enums::*};
use std::{fs, io::ErrorKind, path::PathBuf, sync::Mutex};

lazy_static! {
    pub static ref SYSTEM: System = System::new_all();
    pub static ref PROCESS_EXECUTABLE: String = "steam.exe".to_string();

    pub static ref REGS: Vec<(Mutex<RegKey>, String, String)> = vec![
        (Mutex::new(RegKey::predef(HKEY_CURRENT_USER)), "Software\\Valve".to_string(), "HKEY_CURRENT_USER".to_string()),
        (Mutex::new(RegKey::predef(HKEY_CURRENT_USER)), "Software\\Wow6432Node\\Valve".to_string(), "HKEY_CURRENT_USER".to_string()),
        (Mutex::new(RegKey::predef(HKEY_CURRENT_USER)), "Software\\Classes\\steam".to_string(), "HKEY_CURRENT_USER".to_string()),
        (Mutex::new(RegKey::predef(HKEY_LOCAL_MACHINE)), "Software\\Valve".to_string(), "HKEY_LOCAL_MACHINE".to_string()),
        (Mutex::new(RegKey::predef(HKEY_LOCAL_MACHINE)), "Software\\Wow6432Node\\Valve".to_string(), "HKEY_LOCAL_MACHINE".to_string()),
        (Mutex::new(RegKey::predef(HKEY_LOCAL_MACHINE)), "Software\\Classes\\steam".to_string(), "HKEY_LOCAL_MACHINE".to_string()),
        (Mutex::new(RegKey::predef(HKEY_CLASSES_ROOT)), "steam".to_string(), "HKEY_CLASSES_ROOT".to_string())
    ];

    pub static ref PROGRAM_REGS: Vec<(Mutex<RegKey>, String)> = vec![
        (Mutex::new(RegKey::predef(HKEY_LOCAL_MACHINE)), "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall".to_string()),
        (Mutex::new(RegKey::predef(HKEY_LOCAL_MACHINE)), "Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall".to_string())
    ];

    pub static ref DELETE_DIRS: Vec<String> = vec![
        "appcache".to_string(),
        "logs".to_string(),
        "userdata".to_string(),
        "dumps".to_string(),
        "config".to_string()
    ];

    pub static ref PACKAGE_NAME: String = env!("CARGO_PKG_NAME").to_string();
    pub static ref PACKAGE_VERSION: String = env!("CARGO_PKG_VERSION").to_string();
    pub static ref PACKAGE_AUTHORS: String = env!("CARGO_PKG_AUTHORS").replace(":", " & ").to_string();
}

fn pause() {
    any_key_to_continue::custom_msg(format!("{} Press any key to quit...", "[ EXIT ]".bold().yellow()).as_str());
}

fn is_process_open() -> bool {
    return (*SYSTEM).processes().iter().find(|(_, process)| process.name().to_string_lossy() == *PROCESS_EXECUTABLE).is_some();
}

fn delete_reg(reg: &RegKey, path: &str) -> bool {
    match reg.open_subkey(path) {
        Ok(_) => {
            return reg.delete_subkey_all(path).is_ok();
        },
        _ => { return true; }
    }
}

fn get_installed_programs() -> Vec<(String, String)> {
    let mut programs: Vec<(String, String)> = vec![];

    for (key, path) in PROGRAM_REGS.iter() {
        if let Ok(key) = key.lock() {
            let root_subkey = key.open_subkey(path);

            if let Ok(root_subkey) = root_subkey {
                for subkey in root_subkey.enum_keys() {
                    if let Ok(subkey) = subkey {
                        let opened_subkey = root_subkey.open_subkey_with_flags(subkey, KEY_READ);
                        
                        if let Ok(opened_subkey) = opened_subkey {
                            let display_name = opened_subkey.get_value::<String, String>("DisplayName".to_string());
                            let uninstall_string = opened_subkey.get_value::<String, String>("UninstallString".to_string());

                            if let Ok(display_name) = display_name {
                                if let Ok(uninstall_string) = uninstall_string {
                                    programs.push((display_name, uninstall_string));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    return programs;
}

fn find_steam_directory() -> Option<String> {
    let installed_programs = get_installed_programs();

    if let Some(program) = installed_programs.iter().find(|program| program.0 == "Steam") {
        if let Some(path) = program.1.split("\\uninstall.exe").next() {
            return Some(path.to_string());
        }
    }

    return None;
}

fn delete_directory(path: String) -> bool {
    if fs::metadata(&path).is_err() {
        return true;
    }

    match fs::remove_dir_all(&path) {
        Ok(_) => { return true; },
        Err(err) => { return err.kind() == ErrorKind::NotFound }
    }
}

fn main() {
    set_virtual_terminal(true).unwrap();
    println!("{} {} | {} | {}", "[ INFO ]".bold().cyan(), (*PACKAGE_NAME).to_uppercase().bold(), (*PACKAGE_VERSION).bold(), (*PACKAGE_AUTHORS).bold());

    if user::privileged() {
        println!("{} Running with admin privileges", "[ OKAY ]".bold().green());
    } else {
        println!("{} Running without admin privileges", "[ FAIL ]".bold().red());
        return pause();
    }
    
    match is_process_open() {
        false => {
            println!("{} {} is not open", "[ OKAY ]".bold().green(), (*PROCESS_EXECUTABLE).bold());
        },
        true => {
            println!("{} {} is open", "[ FAIL ]".bold().red(), (*PROCESS_EXECUTABLE).bold());
            return pause();
        }
    }

    let steam_directory = match find_steam_directory() {
        Some(dir) => {
            println!("{} Found steam directory: {}", "[ OKAY ]".bold().green(), dir.bold());
            PathBuf::from(dir)
        },
        _ => {
            println!("{} Failed to find steam directory", "[ FAIL ]".bold().red());
            return pause();
        }
    };
    
    for (key, path, key_string) in REGS.iter() {
        if let Ok(key) = key.lock() {
            match delete_reg(&key, &path) {
                true => {
                    println!("{} Deleted registry {}", "[ OKAY ]".bold().green(), format!("{}\\{}", &key_string, &path).bold());
                },
                false => {
                    println!("{} Failed to delete registry {}", "[ FAIL ]".bold().red(), format!("{}\\{}", &key_string, &path).bold());
                    return pause();
                }
            }
        }
    }

    for path in DELETE_DIRS.iter() {
        let path = steam_directory.join(PathBuf::from(path)).to_string_lossy().to_string();

        match delete_directory(path.clone()) {
            true => {
                println!("{} Deleted directory {}", "[ OKAY ]".bold().green(), path.bold());
            },
            false => {
                println!("{} Failed to delete directory {}", "[ FAIL ]".bold().red(), path.bold());
                return pause();
            }
        }
    }

    pause();
}