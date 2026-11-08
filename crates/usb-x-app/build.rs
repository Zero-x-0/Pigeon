fn main() {
    #[cfg(windows)]
    {
        use std::process::Command;
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_path = format!("{}/app_res.o", out_dir);
        let status = Command::new("windres")
            .current_dir("crates/usb-x-app")
            .args(&["app.rc", "-o", &res_path])
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg={}", res_path);
            }
        }
        println!("cargo:rerun-if-changed=app.manifest");
        println!("cargo:rerun-if-changed=app.rc");
    }
}

