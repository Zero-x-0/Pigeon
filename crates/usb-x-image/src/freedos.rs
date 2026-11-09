pub fn get_freedos_fdconfig_sys() -> &'static str {
    r#"!SET DOSDIR=\FDOS
!LASTDRIVE=Z
!BUFFERS=20
!FILES=40
!DOS=HIGH,UMB
!DOSDATA=UMB
DEVICE=\FDOS\BIN\HIMEMX.EXE
SHELLHIGH=\FDOS\BIN\COMMAND.COM \FDOS\BIN /E:1024 /P=\AUTOEXEC.BAT
"#
}

pub fn get_freedos_autoexec_bat() -> &'static str {
    r#"@ECHO OFF
SET PATH=C:\FDOS\BIN;C:\
SET DOSDIR=C:\FDOS
SET NLSPATH=C:\FDOS\NLS
SET HELPPATH=C:\FDOS\HELP
PROMPT $P$G
ECHO Welcome to FreeDOS (Pigeon Boot Media)
"#
}

