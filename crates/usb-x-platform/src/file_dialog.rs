#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
#[repr(C)]
struct OpenFileNameW {
    l_struct_size: u32,
    hwnd_owner: isize,
    h_instance: isize,
    lpstr_filter: *const u16,
    lpstr_custom_filter: *mut u16,
    n_max_cust_filter: u32,
    n_filter_index: u32,
    lpstr_file: *mut u16,
    n_max_file: u32,
    lpstr_file_title: *mut u16,
    n_max_file_title: u32,
    lpstr_initial_dir: *const u16,
    lpstr_title: *const u16,
    flags: u32,
    n_file_offset: u16,
    n_file_extension: u16,
    lpstr_def_ext: *const u16,
    l_cust_data: isize,
    lpfn_hook: usize,
    lp_template_name: *const u16,
    pv_reserved: *mut std::ffi::c_void,
    dw_reserved: u32,
    flags_ex: u32,
}

#[cfg(windows)]
#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OpenFileNameW) -> i32;
    fn GetSaveFileNameW(lpofn: *mut OpenFileNameW) -> i32;
}

pub fn has_native_file_dialog() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(windows))]
    {
        for binary in &["zenity", "yad", "kdialog"] {
            if let Ok(output) = std::process::Command::new(binary).arg("--version").output() {
                if output.status.success() {
                    return true;
                }
            }
        }
        false
    }
}

pub fn open_file_dialog(title: &str, filter_desc: &str, filter_ext: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let mut filter_utf16: Vec<u16> = Vec::new();
        filter_utf16.extend(OsStr::new(filter_desc).encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new(filter_ext).encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new("All Files (*.*)").encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new("*.*").encode_wide());
        filter_utf16.push(0);
        filter_utf16.push(0);

        let title_utf16: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();

        let mut file_buf = vec![0u16; 2048];

        let mut ofn: OpenFileNameW = unsafe { std::mem::zeroed() };
        ofn.l_struct_size = std::mem::size_of::<OpenFileNameW>() as u32;
        ofn.lpstr_filter = filter_utf16.as_ptr();
        ofn.lpstr_file = file_buf.as_mut_ptr();
        ofn.n_max_file = file_buf.len() as u32;
        ofn.lpstr_title = title_utf16.as_ptr();
        ofn.flags = 0x00000800 | 0x00001000; // OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST

        let ret = unsafe { GetOpenFileNameW(&mut ofn) };
        if ret != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        let (clean_desc, patterns) = parse_linux_filter(filter_desc, filter_ext);

        // 1. Try zenity first (standard on GNOME, XFCE, Kali, Debian, Ubuntu)
        let mut cmd = std::process::Command::new("zenity");
        cmd.arg("--file-selection").arg(format!("--title={}", title));
        if !patterns.is_empty() {
            cmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
            cmd.arg("--file-filter=All Files | *");
        }
        match cmd.output() {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                if !err.trim().is_empty() {
                    let mut retry_cmd = std::process::Command::new("zenity");
                    retry_cmd.arg("--file-selection").arg(format!("--title={}", title));
                    if let Ok(retry_output) = retry_cmd.output() {
                        if retry_output.status.success() {
                            let path = String::from_utf8_lossy(&retry_output.stdout).trim().to_string();
                            if !path.is_empty() {
                                return Some(path);
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }

        // 2. Try yad (Yet Another Dialog, popular on Kali Linux & Debian lightweight)
        let mut ycmd = std::process::Command::new("yad");
        ycmd.arg("--file-selection").arg(format!("--title={}", title));
        if !patterns.is_empty() {
            ycmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
            ycmd.arg("--file-filter=All Files | *");
        }
        if let Ok(output) = ycmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        // 3. Try kdialog (standard on KDE Plasma)
        let mut kcmd = std::process::Command::new("kdialog");
        let kde_filter = if !patterns.is_empty() {
            format!("{}|{}\n*|All Files", patterns, clean_desc)
        } else {
            "*".to_string()
        };
        kcmd.arg("--getopenfilename").arg(".").arg(kde_filter).arg("--title").arg(title);
        if let Ok(output) = kcmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        // 4. Try Python 3 Tkinter fallback
        let py_script = format!(
            "import sys\n\
             try:\n\
                 import tkinter as tk\n\
                 from tkinter import filedialog\n\
                 root = tk.Tk()\n\
                 root.withdraw()\n\
                 res = filedialog.askopenfilename(title={:?})\n\
                 if res: sys.stdout.write(res)\n\
             except Exception:\n\
                 pass",
            title
        );
        let mut py_cmd = std::process::Command::new("python3");
        py_cmd.arg("-c").arg(&py_script);
        if let Ok(output) = py_cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        None
    }
}

pub fn parse_linux_filter(filter_desc: &str, filter_ext: &str) -> (String, String) {
    let clean_desc = if let Some(idx) = filter_desc.find('(') {
        filter_desc[..idx].trim().to_string()
    } else {
        filter_desc.trim().to_string()
    };
    let clean_desc = if clean_desc.is_empty() {
        "Supported Files".to_string()
    } else {
        clean_desc
    };

    let mut patterns = Vec::new();
    for token in filter_ext
        .replace(';', " ")
        .replace(',', " ")
        .replace('|', " ")
        .split_whitespace()
    {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        let norm = if t.starts_with("*.") {
            t.to_string()
        } else if t.starts_with('.') {
            format!("*{}", t)
        } else if t.starts_with('*') {
            t.to_string()
        } else {
            format!("*.{}", t)
        };

        let lower = norm.to_ascii_lowercase();
        let upper = norm.to_ascii_uppercase();

        if !patterns.contains(&lower) {
            patterns.push(lower.clone());
        }
        if lower != upper && !patterns.contains(&upper) {
            patterns.push(upper);
        }
    }

    let pattern_str = patterns.join(" ");
    (clean_desc, pattern_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_linux_filter() {
        let (desc, patterns) = parse_linux_filter(
            "Disk Images (*.iso;*.img;*.bin;*.vhd;*.raw)",
            "*.iso;*.img;*.bin;*.vhd;*.raw",
        );
        assert_eq!(desc, "Disk Images");
        assert!(patterns.contains("*.iso"));
        assert!(patterns.contains("*.ISO"));
        assert!(patterns.contains("*.img"));
        assert!(patterns.contains("*.IMG"));
        assert!(!patterns.contains(';'));
    }
}

pub fn save_file_dialog(title: &str, default_name: &str, filter_desc: &str, filter_ext: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let mut filter_utf16: Vec<u16> = Vec::new();
        filter_utf16.extend(OsStr::new(filter_desc).encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new(filter_ext).encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new("All Files (*.*)").encode_wide());
        filter_utf16.push(0);
        filter_utf16.extend(OsStr::new("*.*").encode_wide());
        filter_utf16.push(0);
        filter_utf16.push(0);

        let title_utf16: Vec<u16> = OsStr::new(title).encode_wide().chain(std::iter::once(0)).collect();

        let mut file_buf = vec![0u16; 2048];
        let def_name_utf16: Vec<u16> = OsStr::new(default_name).encode_wide().collect();
        for (i, &ch) in def_name_utf16.iter().enumerate().take(file_buf.len() - 1) {
            file_buf[i] = ch;
        }

        let mut ofn: OpenFileNameW = unsafe { std::mem::zeroed() };
        ofn.l_struct_size = std::mem::size_of::<OpenFileNameW>() as u32;
        ofn.lpstr_filter = filter_utf16.as_ptr();
        ofn.lpstr_file = file_buf.as_mut_ptr();
        ofn.n_max_file = file_buf.len() as u32;
        ofn.lpstr_title = title_utf16.as_ptr();
        ofn.flags = 0x00000002 | 0x00000800; // OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST

        let ret = unsafe { GetSaveFileNameW(&mut ofn) };
        if ret != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            Some(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            None
        }
    }
    #[cfg(not(windows))]
    {
        let (clean_desc, patterns) = parse_linux_filter(filter_desc, filter_ext);

        // 1. Try zenity
        let mut cmd = std::process::Command::new("zenity");
        cmd.arg("--file-selection").arg("--save").arg(format!("--title={}", title));
        if !default_name.is_empty() {
            cmd.arg(format!("--filename={}", default_name));
        }
        if !patterns.is_empty() {
            cmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
            cmd.arg("--file-filter=All Files | *");
        }
        match cmd.output() {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                if !err.trim().is_empty() {
                    let mut retry_cmd = std::process::Command::new("zenity");
                    retry_cmd.arg("--file-selection").arg("--save").arg(format!("--title={}", title));
                    if !default_name.is_empty() {
                        retry_cmd.arg(format!("--filename={}", default_name));
                    }
                    if let Ok(retry_output) = retry_cmd.output() {
                        if retry_output.status.success() {
                            let path = String::from_utf8_lossy(&retry_output.stdout).trim().to_string();
                            if !path.is_empty() {
                                return Some(path);
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }

        // 2. Try yad
        let mut ycmd = std::process::Command::new("yad");
        ycmd.arg("--file-selection").arg("--save").arg(format!("--title={}", title));
        if !default_name.is_empty() {
            ycmd.arg(format!("--filename={}", default_name));
        }
        if let Ok(output) = ycmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        // 3. Try kdialog
        let mut kcmd = std::process::Command::new("kdialog");
        let kde_filter = if !patterns.is_empty() {
            format!("{}|{}\n*|All Files", patterns, clean_desc)
        } else {
            "*".to_string()
        };
        kcmd.arg("--getsavefilename").arg(default_name).arg(kde_filter).arg("--title").arg(title);
        if let Ok(output) = kcmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        // 4. Try Python 3 Tkinter
        let py_script = format!(
            "import sys\n\
             try:\n\
                 import tkinter as tk\n\
                 from tkinter import filedialog\n\
                 root = tk.Tk()\n\
                 root.withdraw()\n\
                 f = filedialog.asksaveasfilename(title={:?}, initialfile={:?})\n\
                 if f: sys.stdout.write(f)\n\
             except Exception:\n\
                 pass",
            title, default_name
        );
        let mut py_cmd = std::process::Command::new("python3");
        py_cmd.arg("-c").arg(&py_script);
        if let Ok(output) = py_cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }

        None
    }
}

