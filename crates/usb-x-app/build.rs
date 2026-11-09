fn main() {
    #[cfg(windows)]
    {
        use std::process::Command;
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_path = format!("{}/app_res.o", out_dir);

        let status = Command::new("windres")
            .current_dir(&manifest_dir)
            .args(&["app.rc", "-O", "coff", "-o", &res_path])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:rustc-link-arg={}", res_path);
            }
            Ok(s) => {
                panic!("windres failed with exit status: {:?}", s);
            }
            Err(e) => {
                panic!("Failed to spawn windres: {}", e);
            }
        }
        println!("cargo:rerun-if-changed=app.manifest");
        println!("cargo:rerun-if-changed=app.rc");
        println!("cargo:rerun-if-changed=icon.ico");
    }
}
