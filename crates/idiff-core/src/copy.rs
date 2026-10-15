//! Every user-facing string, in one place.

pub const SNAPSHOT_TOO_NEW: &str = "This snapshot was made by a newer idiff. Update idiff to read it.";
pub const SNAPSHOT_UNREADABLE: &str = "This snapshot file is damaged or isn't an idiff snapshot";
/// Why a source couldn't be compared when no error was recorded, as in "Couldn't check {source}: …".
pub const SOURCE_NOT_READ: &str = "it wasn't read in both snapshots";

pub const NO_SESSIONS: &str = "No sessions yet. Run: idiff run -- <install command>";
pub const NO_SUCH_SESSION: &str = "There's no session with that id. See them with: idiff list";
pub const SESSION_UNFINISHED: &str = "This session didn't finish.";
/// Followed by the file's path and what's wrong with it.
pub const NOISE_LOCAL_BAD: &str = "Your local noise list has a mistake, so only the built-in one is used";
/// Followed by the file's path and the system's reason.
pub const CANT_READ_FILE: &str = "Couldn't read";
pub const NOT_BUILT_YET: &str = "not built yet";
/// Why the scheduled tasks list is partial without administrator rights.
pub const TASKS_PARTIAL: &str = "some tasks are only visible to administrators";
/// Why a source that only administrators can read wasn't read, as in "Couldn't check {source}: …".
pub const NEEDS_ADMIN: &str = "needs administrator";
/// The install command couldn't be started because Windows didn't find it.
pub const COMMAND_NOT_FOUND: &str = "That command wasn't found. Check its name, or give its full path.";

/// The subject when nothing says which app was installed.
pub const UNKNOWN_APP: &str = "Unknown app";
/// The note on noise rules that `idiff calibrate --save` writes.
pub const NOTE_CALIBRATE: &str = "calibrate";

// Why a change was given to its owner (`idiff show --why`).
/// Followed by the noise rule's note.
pub const WHY_NOISE: &str = "Windows background change";
pub const WHY_SUBJECT_ENTRY: &str = "the installed program's own entry";
pub const WHY_TARGET_IN_INSTALL_DIR: &str = "runs from the app's folder";
pub const WHY_SIGNER_MATCHES: &str = "signed by the app's publisher";
pub const WHY_COMPANY_MATCHES: &str = "made by the app's publisher, by the file's details";
pub const WHY_ITEM_IN_INSTALL_DIR: &str = "inside the app's folder";
pub const WHY_WINDOWS_COMPONENT: &str = "part of Windows";
pub const WHY_NAME_MATCHES: &str = "name matches the app (a weak sign)";
pub const WHY_HOLDS_APP_PROGRAM: &str = "holds one of the app's programs";
pub const WHY_UNATTRIBUTED: &str = "no rule matched";

pub const SOURCE_PROGRAMS: &str = "installed programs";
pub const SOURCE_SERVICES: &str = "services";
pub const SOURCE_DRIVERS: &str = "drivers";
pub const SOURCE_TASKS: &str = "scheduled tasks";
pub const SOURCE_STARTUP: &str = "startup";
pub const SOURCE_FIREWALL: &str = "firewall rules";
pub const SOURCE_SHELL_EXT: &str = "right-click menu";
pub const SOURCE_PATH: &str = "PATH";
pub const SOURCE_FOLDERS: &str = "folders and shortcuts";
pub const SOURCE_PACKAGED: &str = "Store apps";
pub const SOURCE_CERTS: &str = "trusted root certificates";
pub const SOURCE_DEFENDER: &str = "antivirus exclusions";
pub const SOURCE_NETWORK: &str = "network settings";

// One item of each source, for "Scheduled task "X" is still there" and the change lists.
pub const KIND_PROGRAM: &str = "Program";
pub const KIND_SERVICE: &str = "Service";
pub const KIND_DRIVER: &str = "Driver";
pub const KIND_TASK: &str = "Scheduled task";
pub const KIND_STARTUP: &str = "Startup entry";
pub const KIND_FIREWALL: &str = "Firewall rule";
pub const KIND_SHELL_EXT: &str = "Right-click menu entry";
pub const KIND_PATH: &str = "PATH entry";
pub const KIND_FOLDER: &str = "Folder";
pub const KIND_SHORTCUT: &str = "Shortcut";
pub const KIND_PACKAGED: &str = "Store app";
pub const KIND_CERT: &str = "Trusted root certificate";
pub const KIND_DEFENDER: &str = "Antivirus exclusion";
pub const KIND_NETWORK: &str = "Network setting";

// The receipt (spec §7). `{name}`-style parts are filled in by the verdict.
pub const TITLE_INSTALLED: &str = "{name}: installed";
/// No app was named or found, and nothing changed that could be an install.
pub const TITLE_NO_INSTALL: &str = "No install found";
pub const TITLE_UPDATED: &str = "{name}: updated from {previous} to {version}";
pub const TITLE_LEFTOVERS_NONE: &str = "{name}: after uninstall, no leftovers";
/// When some sources couldn't be checked, so "none" isn't certain.
pub const TITLE_LEFTOVERS_NONE_FOUND: &str = "{name}: after uninstall, no leftovers found";
pub const TITLE_LEFTOVERS_ONE: &str = "{name}: after uninstall, 1 leftover";
pub const TITLE_LEFTOVERS_MANY: &str = "{name}: after uninstall, {n} leftovers";

pub const RESTART_NEEDED: &str = "Needs a restart to finish";
pub const EXIT_CODE: &str = "The install command exited with code {n}";
pub const CERT_ADDED: &str = "Adds a trusted root certificate: \"{subject}\"";
pub const DEFENDER_EXCLUSION: &str = "Excludes {path} from antivirus scans";
pub const NETWORK_CHANGED: &str = "Changes your network settings: {what}";
pub const NETWORK_HOSTS: &str = "hosts file";
pub const NETWORK_PROXY: &str = "proxy";
pub const NETWORK_WINSOCK: &str = "Winsock";
pub const DRIVERS_ONE: &str = "Installs 1 driver: {names}";
pub const DRIVERS_MANY: &str = "Installs {n} drivers: {names}";
pub const SERVICES_ONE: &str = "Adds 1 background service: {names}";
pub const SERVICES_MANY: &str = "Adds {n} background services: {names}";
pub const SERVICE_STARTS_WITH_WINDOWS: &str = " (starts with Windows)";
pub const SERVICE_FULL_RIGHTS: &str = " (full system rights)";
pub const TASKS_ONE: &str = "1 scheduled task: {summary}";
pub const TASKS_MANY: &str = "{n} scheduled tasks: {summary}";
pub const STARTS_WITH_WINDOWS: &str = "Starts with Windows";
pub const FIREWALL_INBOUND: &str = "Allows incoming connections for {program}";
pub const FIREWALL_OUTBOUND: &str = "Adds outbound firewall rules for {program}";
pub const SHELL_EXT_ADDED: &str = "Adds right-click menu entries";
pub const PATH_ADDED: &str = "Adds to PATH: {entries}";
pub const FILES_PROGRAM_FILES: &str = "Files in Program Files ({size})";
pub const FILES_PROGRAM_DATA: &str = "Files in ProgramData ({size})";
pub const FILES_ONLY_USER: &str = "Files only in your user folder ({size})";
/// When the app also has files in Program Files or ProgramData.
pub const FILES_USER: &str = "Files in your user folder ({size})";
pub const SHORTCUT_DESKTOP: &str = "Adds a desktop shortcut";
pub const SHORTCUT_START_MENU: &str = "Adds a Start menu entry";
pub const ALSO_INSTALLED: &str = "Also installed: {names}";
pub const CHANGES_ITEM: &str = "Changes {kind} \"{label}\"";
/// Followed by the absent ones of the three below, as in "No drivers or firewall rules".
pub const NONE_OF: &str = "No {things}";
pub const NONE_SERVICES: &str = "services";
pub const NONE_DRIVERS: &str = "drivers";
pub const NONE_FIREWALL: &str = "firewall rules";
pub const NOTHING_FOUND: &str = "No lasting changes found";
pub const COULDNT_CHECK: &str = "Couldn't check {source}: {reason}";
/// A source that was read, but not all of it (scheduled tasks without administrator rights).
pub const ONLY_PARTLY_CHECKED: &str = "Only partly checked {source}: {reason}";
pub const HIDDEN_ONE: &str = "1 Windows background change hidden (idiff show --all)";
pub const HIDDEN_MANY: &str = "{n} Windows background changes hidden (idiff show --all)";
/// The same count on pages that list the hidden changes themselves (the Receipts Library).
pub const HIDDEN_LISTED_BELOW_ONE: &str = "1 Windows background change hidden (listed below)";
pub const HIDDEN_LISTED_BELOW: &str = "{n} Windows background changes hidden (listed below)";
pub const ALSO_CHANGED: &str = "Also changed while installing (couldn't tell if it was this app)";
pub const CHANGE_ADDED: &str = "Added {kind} \"{label}\"";
pub const CHANGE_REMOVED: &str = "Removed {kind} \"{label}\"";
pub const CHANGE_MODIFIED: &str = "Changed {kind} \"{label}\"";

// Leftovers.
pub const STILL_THERE: &str = "{kind} \"{label}\" is still there";
pub const FOLDER_LEFT: &str = "{size} in {path}";
/// After `STILL_THERE` for a leftover folder with nothing in it.
pub const EMPTY_SUFFIX: &str = " (empty)";
pub const UNINSTALLER_ADDED: &str = "The uninstaller added {kind} \"{label}\"";
pub const NOTHING_LEFT: &str = "Nothing left behind";

// Scheduled task summaries: what it runs, then when.
pub const RUNS_UPDATER: &str = "runs an updater";
pub const RUNS_FILE: &str = "runs {exe}";
/// A task with no program of its own (a COM handler).
pub const RUNS_SOMETHING: &str = "runs a program";
pub const WHEN_DAILY: &str = "every day";
pub const WHEN_WEEKLY: &str = "every week";
pub const WHEN_LOGON: &str = "at sign-in";
pub const WHEN_BOOT: &str = "at startup";
pub const WHEN_IDLE: &str = "when the PC is idle";
pub const WHEN_ONCE: &str = "once";
pub const WHEN_OTHER: &str = "on a schedule";
pub const AS_SYSTEM: &str = ", as SYSTEM";
/// Joins the last two absent things in `NONE_OF`; the others are joined with ", ".
pub const NONE_LAST_JOIN: &str = " or ";

// Rendering.
pub const HIDDEN_HEADING: &str = "Hidden: Windows background changes";
pub const FOOTER: &str = "Made with idiff {version} · it only reads, never changes anything";
/// `idiff diff` works from two files, with nothing to look up on this PC.
pub const DIFF_NOT_ENRICHED: &str = "Compared from files, so signers and folder sizes weren't checked";
// What each icon means, for screen readers in the HTML receipt.
pub const LEVEL_GOOD: &str = "Good:";
pub const LEVEL_NOTE: &str = "Note:";
pub const LEVEL_WARN: &str = "Warning:";
pub const LEVEL_BAD: &str = "Problem:";

// `idiff run`. Progress goes to stderr; the receipt is the only thing on stdout.
pub const FIRST_RUN: &str = "Open {app} once, then close it and press Enter. (Type s and Enter to skip.)";
/// Who `FIRST_RUN` asks to open when nothing names the app yet.
pub const THE_APP: &str = "the app";
pub const RUN_BEFORE: &str = "Reading this PC before the install...";
pub const RUN_STARTING: &str = "Running: {command}";
pub const RUN_SETTLING: &str = "Waiting {n} seconds for the app to settle...";
pub const WAITING_FOR_INSTALLER: &str = "Waiting for the installer to finish...";
pub const RUN_AFTER: &str = "Reading this PC again...";
pub const RUN_TIMED_OUT: &str = "The install command was still running after {n} minutes";
pub const UNINSTALL_TIMED_OUT: &str =
    "The uninstall was still running after {n} minutes, so this list may include things it would have removed";
pub const UNINSTALL_DIDNT_FINISH: &str = "The app is still listed in Installed programs, so the uninstall may not have run or finished; this list may include things it would have removed";
// leftovers, export, calibrate, list
pub const UNINSTALL_NOW: &str = "Uninstall {app} now, then press Enter.";
pub const NOT_REDACTED: &str = "Not redacted: this file contains your user and PC names.";
pub const EXPORTED: &str = "Saved {path}";
pub const CALIBRATE_WATCHING: &str = "Watching for {n} min. Don't install or change anything until it's done.";
pub const CALIBRATE_LEFT: &str = "{time} left";
pub const CALIBRATE_TITLE: &str = "What changed in {n} min with nothing installed";
pub const CALIBRATE_SAVED: &str = "Added {n} rules to {path}";
pub const CALIBRATE_NOTHING_TO_SAVE: &str = "Nothing to add to your noise list";
pub const LIST_FINISHED: &str = "finished";
pub const LIST_UNFINISHED: &str = "didn't finish";
