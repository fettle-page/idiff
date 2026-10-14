use clap::Parser;
use idiff::cli::{Cli, Command, color_enabled};
use idiff::session;
use idiff_core::attribute::attribute;
use idiff_core::copy;
use idiff_core::diff::diff;
use idiff_core::model::Snapshot;
use idiff_core::noise::NoiseList;
use idiff_core::render::{self, TextOptions};
use idiff_core::subject::find_subject;
use idiff_core::verdict::{Level, Line, Receipt, install_receipt};
use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    // An elevated copy started as the helper takes this path, before the normal commands are parsed.
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().is_some_and(|a| a == "--helper") {
            let code = match idiff::elevate::parse_helper_args(&args) {
                Some((path, only)) => idiff::elevate::run_helper(&path, &only),
                None => 1,
            };
            return ExitCode::from(code as u8);
        }
    }
    let result = match Cli::parse().command {
        Command::Run {
            app,
            yes,
            settle,
            timeout,
            no_elevate,
            command,
        } => cmd_run(app.as_deref(), yes, settle, timeout, no_elevate, &command),
        Command::Snapshot { output, no_elevate } => cmd_snapshot(output.as_deref(), no_elevate).map(|()| 0),
        Command::Diff {
            before,
            after,
            app,
            all,
            why,
        } => cmd_diff(&before, &after, app.as_deref(), all, why).map(|()| 0),
        Command::Show {
            session,
            all,
            why,
            json,
        } => cmd_show(&session, all, why, json).map(|()| 0),
        Command::Leftovers {
            session,
            yes,
            settle,
            timeout,
            no_elevate,
            command,
        } => {
            let options = idiff::cmd::leftovers::Options {
                yes,
                settle,
                timeout_mins: timeout,
                no_elevate,
            };
            idiff::cmd::leftovers::run(&session, &options, &command, &noise(), &text_options(false, false))
        }
        Command::Export {
            session,
            format,
            output,
            no_redact,
        } => idiff::cmd::export::run(&session, format, output.as_deref(), no_redact).map(|()| 0),
        Command::Calibrate {
            minutes,
            save,
            no_elevate,
        } => idiff::cmd::calibrate::run(minutes, save, no_elevate, &noise(), &text_options(true, true)).map(|()| 0),
        Command::List => idiff::cmd::list::run().map(|()| 0),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("idiff: {e}");
            ExitCode::from(1)
        }
    }
}

fn load(path: &Path) -> Result<Snapshot, String> {
    let json =
        std::fs::read_to_string(path).map_err(|e| format!("{} {}: {e}", copy::CANT_READ_FILE, path.display()))?;
    Snapshot::load(&json).map_err(|e| format!("{}: {e}", path.display()))
}

fn text_options(all: bool, why: bool) -> TextOptions {
    let terminal = std::io::stdout().is_terminal() && console_shows_colour();
    TextOptions {
        color: color_enabled(terminal, std::env::var_os("NO_COLOR").as_deref()),
        all,
        why,
    }
}

/// Whether the console already turns ANSI codes into colour (Windows Terminal, VS Code and
/// PowerShell switch it on). idiff only reads the console's mode, never changes it.
#[cfg(windows)]
fn console_shows_colour() -> bool {
    use windows::Win32::System::Console::{
        CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let Ok(out) = GetStdHandle(STD_OUTPUT_HANDLE) else {
            return false;
        };
        let mut mode = CONSOLE_MODE(0);
        GetConsoleMode(out, &mut mode).is_ok() && mode.0 & ENABLE_VIRTUAL_TERMINAL_PROCESSING.0 != 0
    }
}

#[cfg(not(windows))]
fn console_shows_colour() -> bool {
    true
}

/// The built-in noise list, plus the user's own `noise.local.toml` if there is one. A local
/// file with a mistake is left out, with a warning on stderr.
fn noise() -> NoiseList {
    let (noise, warning) = idiff::noise::load(&session::home());
    if let Some(warning) = warning {
        eprintln!("idiff: {warning}");
    }
    noise
}

/// To the file given with `-o`, else to stdout. Asks through UAC for the administrator-only
/// sources unless `--no-elevate`.
#[cfg(windows)]
fn cmd_snapshot(output: Option<&Path>, no_elevate: bool) -> Result<(), String> {
    let json = idiff::elevate::snapshot_for_run(no_elevate).to_json();
    match output {
        Some(path) => std::fs::write(path, json).map_err(|e| format!("{}: {e}", path.display())),
        None => {
            println!("{json}");
            Ok(())
        }
    }
}

#[cfg(not(windows))]
fn cmd_snapshot(_output: Option<&Path>, _no_elevate: bool) -> Result<(), String> {
    Err(copy::NOT_BUILT_YET.to_string())
}

/// A receipt from two snapshot files. Signers and folder sizes come from the PC the snapshots
/// describe, which may not be this one, so they aren't looked up; the receipt says so.
fn cmd_diff(before: &Path, after: &Path, app: Option<&str>, all: bool, why: bool) -> Result<(), String> {
    let (before, after) = (load(before)?, load(after)?);
    let delta = diff(&before, &after);
    let subject = find_subject(&delta, app, &[]);
    let attributed = attribute(&delta, &subject, &noise(), &after.env);
    let mut receipt = install_receipt(&subject, &attributed, &delta.notes, &delta.partial, None, &after.env);
    receipt.lines.push(Line {
        level: Level::Note,
        text: copy::DIFF_NOT_ENRICHED.to_string(),
        items: Vec::new(),
        why: Vec::new(),
    });
    print!("{}", render::text(&receipt, &text_options(all, why)));
    Ok(())
}

/// The install receipt, then the leftovers one if `leftovers` was run. `--json` prints the
/// install receipt only (`export --format json` has both).
fn cmd_show(target: &str, all: bool, why: bool, json: bool) -> Result<(), String> {
    let s = session::open(target)?;
    // `run` writes the receipt last, so a run that stopped early has none.
    if !s.dir.join("receipt.json").exists() {
        return Err(copy::SESSION_UNFINISHED.to_string());
    }
    let receipt: Receipt = s.read_json("receipt.json")?;
    if json {
        let text = serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?;
        println!("{text}");
    } else {
        print!("{}", render::text(&receipt, &text_options(all, why)));
        if s.dir.join("leftovers.json").is_file() {
            let leftovers: Receipt = s.read_json("leftovers.json")?;
            print!("\n{}", render::text(&leftovers, &text_options(all, why)));
        }
    }
    Ok(())
}

fn write_step(session: &session::Session, name: &str, value: &impl serde::Serialize) -> Result<(), String> {
    session
        .write_json(name, value)
        .map_err(|e| format!("{}: {e}", session.dir.join(name).display()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Spec §5: snapshot, run the command in a job, wait for it and for Windows Installer, give the
/// app its first run, snapshot again, then diff, enrich, attribute and print the receipt. Each
/// step's file is saved as soon as it exists, so an interrupted run still leaves what it had.
#[cfg(windows)]
fn cmd_run(
    app: Option<&str>,
    yes: bool,
    settle: u64,
    timeout_mins: u64,
    no_elevate: bool,
    command: &[String],
) -> Result<u8, String> {
    use idiff::run::{INSTALLERS_QUIET, exit_status, quote_args, run_in_job, wait_for_installers, wait_for_msi_idle};
    use idiff::session::SessionMeta;
    use idiff_core::subject::winget_id;
    use std::time::{Duration, Instant, SystemTime};

    // Before anything runs, so a broken local noise list can't cost an install.
    let noise = noise();
    let named = app.map(String::from).or_else(|| winget_id(command));
    let hint = named.clone().or_else(|| {
        command
            .first()
            .and_then(|c| Path::new(c).file_stem())
            .map(|s| s.to_string_lossy().into_owned())
    });
    let session = session::create(hint.as_deref().unwrap_or_default()).map_err(|e| e.to_string())?;
    let mut meta: SessionMeta = session.read_json("session.json")?;
    meta.command = command.to_vec();
    meta.app_flag = app.map(String::from);
    write_step(&session, "session.json", &meta)?;

    eprintln!("{}", copy::RUN_BEFORE);
    let before = idiff::elevate::snapshot_for_run(no_elevate);
    write_step(&session, "before.json", &before)?;

    eprintln!("{}", copy::RUN_STARTING.replace("{command}", &quote_args(command)));
    let deadline = Instant::now() + Duration::from_secs(timeout_mins * 60);
    let started = SystemTime::now();
    let mut outcome = run_in_job(command, Duration::from_secs(timeout_mins * 60)).map_err(|e| e.to_string())?;
    eprintln!("{}", copy::WAITING_FOR_INSTALLER);
    // Installers that relaunch themselves outside the job keep working after the command returns.
    // One still at work when time runs out counts as the command running out of time.
    if !wait_for_installers(
        started,
        INSTALLERS_QUIET,
        deadline.saturating_duration_since(Instant::now()),
    ) {
        outcome.timed_out = true;
    }
    wait_for_msi_idle(deadline.saturating_duration_since(Instant::now()));

    let app_name = named.unwrap_or_else(|| copy::THE_APP.to_string());
    first_run(&app_name, yes, settle);

    eprintln!("{}", copy::RUN_AFTER);
    let after = idiff::elevate::snapshot_for_run(no_elevate);
    write_step(&session, "after.json", &after)?;

    let mut delta = diff(&before, &after);
    idiff_collect::enrich::enrich(&mut delta);
    let subject = find_subject(&delta, app, command);
    let attributed = attribute(&delta, &subject, &noise, &after.env);
    let mut receipt = install_receipt(
        &subject,
        &attributed,
        &delta.notes,
        &delta.partial,
        outcome.exit_code,
        &after.env,
    );
    if outcome.timed_out {
        let text = copy::RUN_TIMED_OUT.replace("{n}", &timeout_mins.to_string());
        receipt.lines.insert(
            0,
            Line {
                level: Level::Warn,
                text,
                items: Vec::new(),
                why: Vec::new(),
            },
        );
    }
    write_step(&session, "receipt.json", &receipt)?;
    meta.subject = Some(subject.name.clone());
    meta.exit_code = outcome.exit_code;
    meta.finished_ms = Some(now_ms());
    write_step(&session, "session.json", &meta)?;

    print!("{}", render::text(&receipt, &text_options(false, false)));
    Ok(exit_status(&outcome))
}

#[cfg(not(windows))]
fn cmd_run(_: Option<&str>, _: bool, _: u64, _: u64, _: bool, _: &[String]) -> Result<u8, String> {
    Err(copy::NOT_BUILT_YET.to_string())
}

/// Spec §5 step 3: changes many apps make only when they first start (a startup entry, an
/// updater task) should be on the receipt too. With `--yes` there's nobody to ask, so wait.
#[cfg(windows)]
fn first_run(app: &str, yes: bool, settle: u64) {
    use std::io::Write;
    if yes {
        if settle > 0 {
            eprintln!("{}", copy::RUN_SETTLING.replace("{n}", &settle.to_string()));
            std::thread::sleep(std::time::Duration::from_secs(settle));
        }
        return;
    }
    eprint!("{} ", copy::FIRST_RUN.replace("{app}", app));
    let _ = std::io::stderr().flush();
    // Enter or "s" and Enter both go on: skipping only means the app wasn't opened.
    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
}
