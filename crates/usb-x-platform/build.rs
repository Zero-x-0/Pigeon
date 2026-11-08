fn main() {
    #[cfg(windows)]
    {
        cc::Build::new()
            .file("c_src/disk_lowlevel.c")
            .file("c_src/fast_hash.c")
            .include("c_src")
            .opt_level(3)
            .compile("usb_x_lowlevel_c");

        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rerun-if-changed=c_src/disk_lowlevel.h");
        println!("cargo:rerun-if-changed=c_src/disk_lowlevel.c");
        println!("cargo:rerun-if-changed=c_src/fast_hash.h");
        println!("cargo:rerun-if-changed=c_src/fast_hash.c");
    }
}

