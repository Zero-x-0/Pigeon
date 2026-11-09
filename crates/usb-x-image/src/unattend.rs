use usb_x_core::types::Windows11Tweaks;

pub fn generate_autounattend_xml(tweaks: &Windows11Tweaks) -> String {
    generate_autounattend_xml_with_arch(tweaks, "amd64")
}

pub fn generate_autounattend_xml_with_arch(tweaks: &Windows11Tweaks, raw_arch: &str) -> String {
    let arch = {
        let l = raw_arch.to_ascii_lowercase();
        if l.contains("86") || l.contains("32") {
            "x86"
        } else if l.contains("arm") {
            "arm64"
        } else {
            "amd64"
        }
    };

    let mut run_synch_commands = Vec::new();
    let mut order = 1;

    // 1. TPM, Secure Boot, RAM, CPU bypass via LabConfig in WinPE phase
    if tweaks.bypass_tpm_secure_boot_ram {
        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassTPMCheck /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass TPM 2.0 Check</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;

        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassSecureBootCheck /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass Secure Boot Check</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;

        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassRAMCheck /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass RAM Check</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;

        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassCPUCheck /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass CPU Requirement Check</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;

        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassStorageCheck /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass Storage Check</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;
    }

    // 2. Microsoft Account requirement bypass (BypassNRO)
    if tweaks.bypass_online_account {
        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\OOBE /v BypassNRO /t REG_DWORD /d 1 /f</Path>
                    <Description>Bypass Microsoft Online Account Requirement</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;
    }

    // 3. Disable automatic BitLocker device encryption
    if tweaks.disable_bitlocker_device_encryption {
        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SYSTEM\CurrentControlSet\Control\BitLocker /v PreventDeviceEncryption /t REG_DWORD /d 1 /f</Path>
                    <Description>Disable Automatic BitLocker Encryption</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;
    }

    // 4. Privacy tweaks: disable telemetry and diagnostic data collection
    if tweaks.disable_telemetry_privacy {
        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection /v AllowTelemetry /t REG_DWORD /d 0 /f</Path>
                    <Description>Disable Telemetry Data Collection</Description>
                </RunSynchronousCommand>"#,
            order
        ));
        order += 1;

        run_synch_commands.push(format!(
            r#"                <RunSynchronousCommand wcm:action="add">
                    <Order>{}</Order>
                    <Path>reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AdvertisingInfo /v Enabled /t REG_DWORD /d 0 /f</Path>
                    <Description>Disable Advertising ID</Description>
                </RunSynchronousCommand>"#,
            order
        ));
    }

    let win_pe_commands = run_synch_commands.join("\n");
    let run_synchronous_block = if win_pe_commands.trim().is_empty() {
        String::new()
    } else {
        format!(
            "            <RunSynchronous>\n{}\n            </RunSynchronous>\n",
            win_pe_commands
        )
    };

    let username = if tweaks.local_username.trim().is_empty() {
        "User"
    } else {
        tweaks.local_username.trim()
    };

    let user_accounts_block = if tweaks.create_local_account {
        format!(
            r#"        <component name="Microsoft-Windows-Shell-Setup" processorArchitecture="{arch}" publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <UserAccounts>
                <LocalAccounts>
                    <LocalAccount wcm:action="add">
                        <Description>Pigeon Local User</Description>
                        <DisplayName>{username}</DisplayName>
                        <Group>Administrators</Group>
                        <Name>{username}</Name>
                    </LocalAccount>
                </LocalAccounts>
            </UserAccounts>
            <OOBE>
                <HideEULAPage>true</HideEULAPage>
                <HideLocalAccountScreen>true</HideLocalAccountScreen>
                <HideOnlineAccountScreens>true</HideOnlineAccountScreens>
                <HideWirelessSetupInOOBE>true</HideWirelessSetupInOOBE>
                <ProtectYourPC>3</ProtectYourPC>
            </OOBE>
        </component>"#
        )
    } else {
        String::new()
    };

    let oobe_settings_block = if user_accounts_block.trim().is_empty() {
        String::new()
    } else {
        format!("    <settings pass=\"oobeSystem\">\n{}\n    </settings>\n", user_accounts_block)
    };

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend">
    <settings pass="windowsPE">
        <component name="Microsoft-Windows-Setup" processorArchitecture="{arch}" publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <UserData>
                <AcceptEula>true</AcceptEula>
                <ProductKey>
                    <Key />
                </ProductKey>
            </UserData>
{run_synchronous_block}        </component>
    </settings>
{oobe_settings_block}</unattend>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autounattend_tpm_bypass() {
        let mut tweaks = Windows11Tweaks::default();
        tweaks.bypass_tpm_secure_boot_ram = true;
        tweaks.create_local_account = true;
        tweaks.local_username = "DevUser".to_string();

        let xml = generate_autounattend_xml(&tweaks);
        assert!(xml.contains("BypassTPMCheck"));
        assert!(xml.contains("BypassSecureBootCheck"));
        assert!(xml.contains("<UserData>"));
        assert!(xml.contains("<AcceptEula>true</AcceptEula>"));
        assert!(xml.contains("<ProductKey>"));
        assert!(xml.contains("<Key />"));
        assert!(xml.contains("<DisplayName>DevUser</DisplayName>"));
        assert!(xml.contains("<Name>DevUser</Name>"));
    }
}

