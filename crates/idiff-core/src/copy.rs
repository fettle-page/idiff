//! Every user-facing string, in one place.

pub const SNAPSHOT_TOO_NEW: &str = "This snapshot was made by a newer idiff. Update idiff to read it.";
pub const SNAPSHOT_UNREADABLE: &str = "This snapshot file is damaged or isn't an idiff snapshot";
/// Why a source couldn't be compared when no error was recorded, as in "Couldn't check {source}: …".
pub const SOURCE_NOT_READ: &str = "it wasn't read in both snapshots";

pub const NO_SESSIONS: &str = "No sessions yet. Run: idiff run -- <install command>";
pub const NO_SUCH_SESSION: &str = "There's no session with that id. See them with: idiff list";
/// Followed by the file's path and the system's reason.
pub const CANT_READ_FILE: &str = "Couldn't read";
pub const NO_CHANGES: &str = "No changes";
pub const NOT_BUILT_YET: &str = "not built yet";
/// Why the scheduled tasks list is partial without administrator rights.
pub const TASKS_PARTIAL: &str = "some tasks are only visible to administrators";

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
