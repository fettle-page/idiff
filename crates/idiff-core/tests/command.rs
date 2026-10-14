use idiff_core::command::{extract_exe, normalize_path};
use idiff_core::model::Env;

fn env_fixture() -> Env {
    Env::from([
        ("SystemRoot".into(), r"C:\Windows".into()),
        ("ProgramFiles".into(), r"C:\Program Files".into()),
        ("LOCALAPPDATA".into(), r"C:\Users\a\AppData\Local".into()),
    ])
}

#[test]
fn exes_are_found_in_command_lines() {
    let env = env_fixture();
    for (input, want) in [
        (
            r#""C:\Program Files\X\x.exe" --flag"#,
            Some(r"c:\program files\x\x.exe"),
        ),
        (r"C:\Program Files\X Y\x.exe -a", Some(r"c:\program files\x y\x.exe")),
        (
            r"%LOCALAPPDATA%\Discord\Update.exe --processStart Discord.exe",
            Some(r"c:\users\a\appdata\local\discord\update.exe"),
        ),
        (r"rundll32.exe C:\X\y.dll,Entry", Some(r"c:\x\y.dll")),
        (
            r"\SystemRoot\System32\drivers\foo.sys",
            Some(r"c:\windows\system32\drivers\foo.sys"),
        ),
        (
            r"system32\svchost.exe -k netsvcs",
            Some(r"c:\windows\system32\svchost.exe"),
        ),
        (r"\??\C:\Drivers\bar.sys", Some(r"c:\drivers\bar.sys")),
        ("", None),
    ] {
        assert_eq!(extract_exe(input, &env).as_deref(), want, "{input}");
    }
}

#[test]
fn launchers_give_the_file_they_run() {
    let env = env_fixture();
    for (input, want) in [
        (
            r#"C:\Windows\System32\rundll32.exe "C:\X Y\z.dll",Run"#,
            Some(r"c:\x y\z.dll"),
        ),
        (r"regsvr32.exe /s C:\X\ext.dll", Some(r"c:\x\ext.dll")),
        (r#"cmd.exe /d /c "C:\X Y\run.bat" arg"#, Some(r"c:\x y\run.bat")),
        (r"cmd /C C:\X\run.cmd", Some(r"c:\x\run.cmd")),
        (
            r"powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\X\job.ps1 -Quiet",
            Some(r"c:\x\job.ps1"),
        ),
        (r#"pwsh -f "C:\X Y\job.ps1""#, Some(r"c:\x y\job.ps1")),
        (r"powershell.exe -Command Get-Date", Some("powershell.exe")),
        (r"wscript.exe //B //Nologo C:\X\hide.vbs", Some(r"c:\x\hide.vbs")),
        (r"rundll32.exe", Some("rundll32.exe")),
    ] {
        assert_eq!(extract_exe(input, &env).as_deref(), want, "{input}");
    }
}

#[test]
fn a_command_without_a_known_extension_gives_its_first_word() {
    let env = env_fixture();
    assert_eq!(
        extract_exe(r"C:\Tools\agent --run", &env).as_deref(),
        Some(r"c:\tools\agent")
    );
    assert_eq!(extract_exe("   ", &env), None);
}

#[test]
fn paths_are_normalized() {
    let env = env_fixture();
    for (input, want) in [
        (r#"  "C:/Program Files/X/"  "#, r"c:\program files\x\"),
        (r"%programfiles%\X", r"c:\program files\x"),
        (r"%NotSet%\X", r"%notset%\x"),
        (r"\SystemRoot\x.sys", r"c:\windows\x.sys"),
        (r"\??\C:\Niccolò\À.sys", r"c:\niccolò\à.sys"),
        (r"System32\drivers\x.sys", r"c:\windows\system32\drivers\x.sys"),
    ] {
        assert_eq!(normalize_path(input, &env), want, "{input}");
    }
}

#[test]
fn windir_means_system_root() {
    assert_eq!(
        extract_exe(r"%windir%\system32\SecurityHealthSystray.exe", &env_fixture()).as_deref(),
        Some(r"c:\windows\system32\securityhealthsystray.exe")
    );
}
