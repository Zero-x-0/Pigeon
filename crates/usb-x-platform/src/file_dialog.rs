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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    Selected(String),
    Cancelled,
    Unsupported,
}

#[cfg(not(windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDesktopEnv {
    KdeDolphin,
    XfceThunar,
    GnomeNautilus,
    CinnamonNemo,
    Generic,
}

#[cfg(not(windows))]
fn is_command_available(cmd: &str) -> bool {
    // 1. Check directories in PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            if !dir.is_empty() {
                let p = std::path::Path::new(dir).join(cmd);
                if p.is_file() {
                    return true;
                }
            }
        }
    }
    // 2. Check standard system locations directly
    for dir in &["/usr/bin", "/bin", "/usr/local/bin", "/usr/sbin", "/sbin"] {
        if std::path::Path::new(dir).join(cmd).is_file() {
            return true;
        }
    }
    // 3. Fallback: try executing --version
    if let Ok(output) = std::process::Command::new(cmd).arg("--version").output() {
        return output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty();
    }
    false
}

#[cfg(not(windows))]
fn configure_cmd_env(cmd: &mut std::process::Command) {
    if std::env::var("DISPLAY").is_err() {
        cmd.env("DISPLAY", ":0");
    }

    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        if !sudo_user.is_empty() && sudo_user != "root" {
            let user_home = format!("/home/{}", sudo_user);

            if std::env::var("XAUTHORITY").is_err() {
                let xauth_path = format!("{}/.Xauthority", user_home);
                if std::path::Path::new(&xauth_path).is_file() {
                    cmd.env("XAUTHORITY", xauth_path);
                }
            }

            if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
                if let Ok(output) = std::process::Command::new("id").arg("-u").arg(&sudo_user).output() {
                    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let bus_path = format!("/run/user/{}/bus", uid);
                    if std::path::Path::new(&bus_path).exists() {
                        cmd.env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", bus_path));
                    }
                }
            }

            cmd.env("HOME", &user_home);
            cmd.env("USER", &sudo_user);
            cmd.env("LOGNAME", &sudo_user);
        }
    }
}

#[cfg(not(windows))]
fn detect_desktop_env() -> LinuxDesktopEnv {
    let xdg_current = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_uppercase();
    let desktop_session = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_uppercase();

    let has_dolphin = is_command_available("dolphin");
    let has_thunar = is_command_available("thunar");
    let has_nautilus = is_command_available("nautilus");
    let has_nemo = is_command_available("nemo");

    if xdg_current.contains("KDE") || desktop_session.contains("KDE") || desktop_session.contains("PLASMA") || has_dolphin {
        LinuxDesktopEnv::KdeDolphin
    } else if xdg_current.contains("XFCE") || desktop_session.contains("XFCE") || has_thunar {
        LinuxDesktopEnv::XfceThunar
    } else if xdg_current.contains("GNOME") || desktop_session.contains("GNOME") || desktop_session.contains("UBUNTU") || has_nautilus {
        LinuxDesktopEnv::GnomeNautilus
    } else if xdg_current.contains("CINNAMON") || desktop_session.contains("CINNAMON") || has_nemo {
        LinuxDesktopEnv::CinnamonNemo
    } else {
        LinuxDesktopEnv::Generic
    }
}

pub fn has_native_file_dialog() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(windows))]
    {
        for binary in &["zenity", "yad", "kdialog", "qarma"] {
            if is_command_available(binary) {
                return true;
            }
        }
        if is_command_available("python3") {
            return true;
        }
        false
    }
}

pub fn open_file_dialog(title: &str, filter_desc: &str, filter_ext: &str) -> Option<String> {
    match open_file_dialog_ex(title, filter_desc, filter_ext) {
        DialogResult::Selected(path) => Some(path),
        _ => None,
    }
}

pub fn open_file_dialog_ex(title: &str, filter_desc: &str, filter_ext: &str) -> DialogResult {
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
            DialogResult::Selected(String::from_utf16_lossy(&file_buf[..len]))
        } else {
            DialogResult::Cancelled
        }
    }
    #[cfg(not(windows))]
    {
        let (clean_desc, patterns) = parse_linux_filter(filter_desc, filter_ext);
        let kde_filter = if !patterns.is_empty() {
            format!("{}|{}\n*|All Files", patterns, clean_desc)
        } else {
            "*".to_string()
        };

        let env = detect_desktop_env();

        type DialogRunner = Box<dyn Fn() -> DialogResult>;
        let runners: Vec<DialogRunner> = match env {
            LinuxDesktopEnv::KdeDolphin => vec![
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_kdialog_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_python_qt_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_zenity_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_yad_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_python_gtk_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    move || run_python_tkinter_open(&t)
                }),
            ],
            LinuxDesktopEnv::XfceThunar | LinuxDesktopEnv::GnomeNautilus | LinuxDesktopEnv::CinnamonNemo => vec![
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_zenity_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_yad_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_python_gtk_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_kdialog_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_python_qt_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    move || run_python_tkinter_open(&t)
                }),
            ],
            LinuxDesktopEnv::Generic => vec![
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_zenity_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_yad_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let cd = clean_desc.clone();
                    let pt = patterns.clone();
                    move || run_python_gtk_open(&t, &cd, &pt)
                }),
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_kdialog_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    let kf = kde_filter.clone();
                    move || run_python_qt_open(&t, &kf)
                }),
                Box::new({
                    let t = title.to_string();
                    move || run_python_tkinter_open(&t)
                }),
            ],
        };

        for runner in runners {
            let res = runner();
            match res {
                DialogResult::Selected(path) => return DialogResult::Selected(path),
                DialogResult::Cancelled => return DialogResult::Cancelled,
                DialogResult::Unsupported => continue,
            }
        }

        DialogResult::Unsupported
    }
}

#[cfg(not(windows))]
fn run_zenity_open(title: &str, clean_desc: &str, patterns: &str) -> DialogResult {
    if !is_command_available("zenity") {
        return DialogResult::Unsupported;
    }
    let mut cmd = std::process::Command::new("zenity");
    configure_cmd_env(&mut cmd);
    cmd.arg("--file-selection").arg(format!("--title={}", title));
    if !patterns.is_empty() {
        cmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
        cmd.arg("--file-filter=All Files | *");
    }
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else if output.status.code() == Some(1) {
                DialogResult::Cancelled
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
    }
}

#[cfg(not(windows))]
fn run_yad_open(title: &str, clean_desc: &str, patterns: &str) -> DialogResult {
    if !is_command_available("yad") {
        return DialogResult::Unsupported;
    }
    let mut cmd = std::process::Command::new("yad");
    configure_cmd_env(&mut cmd);
    cmd.arg("--file-selection").arg(format!("--title={}", title));
    if !patterns.is_empty() {
        cmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
        cmd.arg("--file-filter=All Files | *");
    }
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else if output.status.code() == Some(1) {
                DialogResult::Cancelled
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
    }
}

#[cfg(not(windows))]
fn run_kdialog_open(title: &str, kde_filter: &str) -> DialogResult {
    if !is_command_available("kdialog") {
        return DialogResult::Unsupported;
    }
    let mut cmd = std::process::Command::new("kdialog");
    configure_cmd_env(&mut cmd);
    cmd.arg("--getopenfilename").arg(".").arg(kde_filter).arg("--title").arg(title);
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else if output.status.code() == Some(1) {
                DialogResult::Cancelled
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
    }
}

#[cfg(not(windows))]
fn run_python_gtk_open(title: &str, clean_desc: &str, patterns: &str) -> DialogResult {
    if !is_command_available("python3") {
        return DialogResult::Unsupported;
    }
    let script = format!(
        r#"import sys
try:
    import gi
    gi.require_version('Gtk', '3.0')
    from gi.repository import Gtk
    dialog = Gtk.FileChooserDialog(
        title={title:?},
        action=Gtk.FileChooserAction.OPEN
    )
    dialog.add_buttons(
        "_Cancel", Gtk.ResponseType.CANCEL,
        "_Open", Gtk.ResponseType.ACCEPT
    )
    dialog.set_default_response(Gtk.ResponseType.ACCEPT)
    dialog.set_select_multiple(False)
    
    f1 = Gtk.FileFilter()
    f1.set_name({clean_desc:?})
    for pat in {patterns:?}.split():
        if pat:
            f1.add_pattern(pat)
    dialog.add_filter(f1)

    f2 = Gtk.FileFilter()
    f2.set_name("All Files (*.*)")
    f2.add_pattern("*")
    dialog.add_filter(f2)

    resp = dialog.run()
    if resp == Gtk.ResponseType.ACCEPT:
        fn = dialog.get_filename()
        if fn:
            sys.stdout.write(fn)
    dialog.destroy()
    while Gtk.events_pending():
        Gtk.main_iteration()
    sys.exit(0)
except Exception:
    sys.exit(2)
"#,
        title = title,
        clean_desc = clean_desc,
        patterns = patterns
    );

    let mut cmd = std::process::Command::new("python3");
    configure_cmd_env(&mut cmd);
    cmd.arg("-c").arg(&script);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
    }
}

#[cfg(not(windows))]
fn run_python_qt_open(title: &str, kde_filter: &str) -> DialogResult {
    if !is_command_available("python3") {
        return DialogResult::Unsupported;
    }
    let script = format!(
        r#"import sys
def try_qt():
    for mod in ['PyQt5', 'PyQt6', 'PySide2', 'PySide6']:
        try:
            widgets = __import__(mod + '.QtWidgets', fromlist=['QApplication', 'QFileDialog'])
            QApp = getattr(widgets, 'QApplication')
            QFD = getattr(widgets, 'QFileDialog')
            app = QApp.instance() or QApp(sys.argv)
            res = QFD.getOpenFileName(None, {title:?}, "", {kde_filter:?})
            fn = res[0] if isinstance(res, tuple) else res
            if fn:
                sys.stdout.write(str(fn))
            sys.exit(0)
        except Exception:
            continue
    sys.exit(2)
try_qt()
"#,
        title = title,
        kde_filter = kde_filter
    );

    let mut cmd = std::process::Command::new("python3");
    configure_cmd_env(&mut cmd);
    cmd.arg("-c").arg(&script);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
    }
}

#[cfg(not(windows))]
fn run_python_tkinter_open(title: &str) -> DialogResult {
    if !is_command_available("python3") {
        return DialogResult::Unsupported;
    }
    let script = format!(
        r#"import sys
try:
    import tkinter as tk
    from tkinter import filedialog
    root = tk.Tk()
    root.withdraw()
    res = filedialog.askopenfilename(title={title:?})
    if res:
        sys.stdout.write(res)
    sys.exit(0)
except Exception:
    sys.exit(2)
"#,
        title = title
    );

    let mut cmd = std::process::Command::new("python3");
    configure_cmd_env(&mut cmd);
    cmd.arg("-c").arg(&script);

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    DialogResult::Selected(path)
                } else {
                    DialogResult::Cancelled
                }
            } else {
                DialogResult::Unsupported
            }
        }
        Err(_) => DialogResult::Unsupported,
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
        let kde_filter = if !patterns.is_empty() {
            format!("{}|{}\n*|All Files", patterns, clean_desc)
        } else {
            "*".to_string()
        };

        let env = detect_desktop_env();

        let try_zenity = || {
            if !is_command_available("zenity") { return None; }
            let mut cmd = std::process::Command::new("zenity");
            configure_cmd_env(&mut cmd);
            cmd.arg("--file-selection").arg("--save").arg(format!("--title={}", title));
            if !default_name.is_empty() {
                cmd.arg(format!("--filename={}", default_name));
            }
            if !patterns.is_empty() {
                cmd.arg(format!("--file-filter={} | {}", clean_desc, patterns));
                cmd.arg("--file-filter=All Files | *");
            }
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            None
        };

        let try_yad = || {
            if !is_command_available("yad") { return None; }
            let mut ycmd = std::process::Command::new("yad");
            configure_cmd_env(&mut ycmd);
            ycmd.arg("--file-selection").arg("--save").arg(format!("--title={}", title));
            if !default_name.is_empty() {
                ycmd.arg(format!("--filename={}", default_name));
            }
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
            None
        };

        let try_kdialog = || {
            if !is_command_available("kdialog") { return None; }
            let mut kcmd = std::process::Command::new("kdialog");
            configure_cmd_env(&mut kcmd);
            kcmd.arg("--getsavefilename").arg(default_name).arg(&kde_filter).arg("--title").arg(title);
            if let Ok(output) = kcmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            None
        };

        let try_python_gtk = || {
            if !is_command_available("python3") { return None; }
            let script = format!(
                r#"import sys
try:
    import gi
    gi.require_version('Gtk', '3.0')
    from gi.repository import Gtk
    dialog = Gtk.FileChooserDialog(
        title={title:?},
        action=Gtk.FileChooserAction.SAVE
    )
    dialog.add_buttons(
        "_Cancel", Gtk.ResponseType.CANCEL,
        "_Save", Gtk.ResponseType.ACCEPT
    )
    dialog.set_default_response(Gtk.ResponseType.ACCEPT)
    dialog.set_do_overwrite_confirmation(True)
    if {default_name:?}:
        dialog.set_current_name({default_name:?})

    f1 = Gtk.FileFilter()
    f1.set_name({clean_desc:?})
    for pat in {patterns:?}.split():
        if pat:
            f1.add_pattern(pat)
    dialog.add_filter(f1)

    resp = dialog.run()
    if resp == Gtk.ResponseType.ACCEPT:
        fn = dialog.get_filename()
        if fn:
            sys.stdout.write(fn)
    dialog.destroy()
    while Gtk.events_pending():
        Gtk.main_iteration()
    sys.exit(0)
except Exception:
    sys.exit(2)
"#,
                title = title,
                default_name = default_name,
                clean_desc = clean_desc,
                patterns = patterns
            );

            let mut cmd = std::process::Command::new("python3");
            configure_cmd_env(&mut cmd);
            cmd.arg("-c").arg(&script);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            None
        };

        let try_python_qt = || {
            if !is_command_available("python3") { return None; }
            let script = format!(
                r#"import sys
def try_qt():
    for mod in ['PyQt5', 'PyQt6', 'PySide2', 'PySide6']:
        try:
            widgets = __import__(mod + '.QtWidgets', fromlist=['QApplication', 'QFileDialog'])
            QApp = getattr(widgets, 'QApplication')
            QFD = getattr(widgets, 'QFileDialog')
            app = QApp.instance() or QApp(sys.argv)
            res = QFD.getSaveFileName(None, {title:?}, {default_name:?}, {kde_filter:?})
            fn = res[0] if isinstance(res, tuple) else res
            if fn:
                sys.stdout.write(str(fn))
            sys.exit(0)
        except Exception:
            continue
    sys.exit(2)
try_qt()
"#,
                title = title,
                default_name = default_name,
                kde_filter = kde_filter
            );

            let mut cmd = std::process::Command::new("python3");
            configure_cmd_env(&mut cmd);
            cmd.arg("-c").arg(&script);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            None
        };

        let try_python_tkinter = || {
            if !is_command_available("python3") { return None; }
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
            let mut cmd = std::process::Command::new("python3");
            configure_cmd_env(&mut cmd);
            cmd.arg("-c").arg(&py_script);

            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(path);
                    }
                }
            }
            None
        };

        match env {
            LinuxDesktopEnv::KdeDolphin => {
                if let Some(p) = try_kdialog() { return Some(p); }
                if let Some(p) = try_python_qt() { return Some(p); }
                if let Some(p) = try_zenity() { return Some(p); }
                if let Some(p) = try_yad() { return Some(p); }
                if let Some(p) = try_python_gtk() { return Some(p); }
                try_python_tkinter()
            }
            _ => {
                if let Some(p) = try_zenity() { return Some(p); }
                if let Some(p) = try_yad() { return Some(p); }
                if let Some(p) = try_python_gtk() { return Some(p); }
                if let Some(p) = try_kdialog() { return Some(p); }
                if let Some(p) = try_python_qt() { return Some(p); }
                try_python_tkinter()
            }
        }
    }
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
