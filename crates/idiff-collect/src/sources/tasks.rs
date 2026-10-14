//! Scheduled tasks, read from the Task Scheduler and parsed from each task's XML.

use idiff_core::command::extract_exe;
use idiff_core::model::Item;
use roxmltree::{Document, Node};
use std::collections::BTreeMap;

fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    node.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text<'a>(node: Option<Node<'a, '_>>) -> Option<&'a str> {
    node.and_then(|n| n.text()).map(str::trim).filter(|t| !t.is_empty())
}

/// What starts the task, in words the receipt can use: `Daily`, `Logon`, …
fn trigger_name(trigger: Node) -> String {
    let schedule = |name| child(trigger, name).is_some();
    match trigger.tag_name().name() {
        "CalendarTrigger" if schedule("ScheduleByDay") => "Daily",
        "CalendarTrigger" if schedule("ScheduleByWeek") => "Weekly",
        "CalendarTrigger" if schedule("ScheduleByMonth") || schedule("ScheduleByMonthDayOfWeek") => "Monthly",
        "LogonTrigger" => "Logon",
        "BootTrigger" => "Boot",
        "IdleTrigger" => "Idle",
        "TimeTrigger" => "Time",
        other => other,
    }
    .to_string()
}

/// One registered task. `path` is its full path in the Task Scheduler (`\Folder\Name`).
pub fn task_item(path: &str, xml: &str) -> Result<Item, String> {
    let doc = Document::parse(xml).map_err(|e| format!("{path}: {e}"))?;
    let task = doc.root_element();
    if task.tag_name().name() != "Task" {
        return Err(format!("{path}: not a task definition"));
    }

    let mut triggers: Vec<String> = Vec::new();
    for trigger in child(task, "Triggers")
        .into_iter()
        .flat_map(|t| t.children())
        .filter(Node::is_element)
    {
        let name = trigger_name(trigger);
        if !triggers.contains(&name) {
            triggers.push(name);
        }
    }

    // The principal the actions run as, else the first one.
    let actions = child(task, "Actions");
    let context = actions.and_then(|a| a.attribute("Context"));
    let principal = child(task, "Principals").and_then(|p| {
        let all = || {
            p.children()
                .filter(|c| c.is_element() && c.tag_name().name() == "Principal")
        };
        all()
            .find(|c| context.is_some() && c.attribute("id") == context)
            .or_else(|| all().next())
    });
    let user = principal.and_then(|p| text(child(p, "UserId")).or_else(|| text(child(p, "GroupId"))));
    let run_level = principal.and_then(|p| text(child(p, "RunLevel")));

    let exec = actions.and_then(|a| child(a, "Exec"));
    let command = exec.and_then(|e| text(child(e, "Command")));
    let args = exec.and_then(|e| text(child(e, "Arguments")));
    let com = actions
        .and_then(|a| child(a, "ComHandler"))
        .and_then(|c| text(child(c, "ClassId")));
    // Quoted, so a launcher (rundll32, cmd /c, powershell -File) still yields the file it runs.
    let target = command.and_then(|c| {
        let line = format!("\"{}\" {}", c.trim_matches('"'), args.unwrap_or(""));
        extract_exe(&line, crate::env())
    });
    let enabled = text(child(task, "Settings").and_then(|s| child(s, "Enabled"))).unwrap_or("true");
    let triggers = (!triggers.is_empty()).then(|| triggers.join(","));

    let props = [
        ("triggers", triggers.as_deref()),
        ("user", user),
        ("run_level", run_level),
        ("enabled", Some(enabled)),
        ("args", args),
        ("com", com),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k.to_string(), v.to_string())))
    .collect::<BTreeMap<_, _>>();

    Ok(Item {
        key: path.to_lowercase(),
        label: path.to_string(),
        target,
        props,
    })
}

#[cfg(windows)]
pub use reader::read;

#[cfg(windows)]
mod reader {
    use super::task_item;
    use crate::SourceOutput;
    use idiff_core::copy;
    use idiff_core::model::Item;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    };
    use windows::Win32::System::TaskScheduler::{ITaskFolder, ITaskService, TASK_ENUM_HIDDEN, TaskScheduler};
    use windows::Win32::System::Variant::VARIANT;
    use windows::core::BSTR;

    /// Undoes this thread's CoInitializeEx when the read ends, however it ends.
    struct Com;

    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    /// Every task in every folder, hidden ones included. Without administrator rights some
    /// folders and tasks can't be seen, so the list is marked partial.
    pub fn read() -> Result<SourceOutput, String> {
        let _com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map(|()| Com)
            .map_err(|e| format!("COM: {e}"))?;
        let scheduler = |e: windows::core::Error| format!("Task Scheduler: {e}");
        let service: ITaskService =
            unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER) }.map_err(scheduler)?;
        let none = VARIANT::default();
        unsafe { service.Connect(&none, &none, &none, &none) }.map_err(scheduler)?;
        let root = unsafe { service.GetFolder(&BSTR::from(r"\")) }.map_err(scheduler)?;

        let mut items = Vec::new();
        let mut errors = Vec::new();
        walk(&root, &mut items, &mut errors);
        items.sort_by(|a, b| a.key.cmp(&b.key));

        let note = if crate::elevation::is_elevated() {
            (!errors.is_empty()).then(|| format!("Couldn't read {}", errors.join("; ")))
        } else {
            Some(copy::TASKS_PARTIAL.to_string())
        };
        Ok(SourceOutput { items, note })
    }

    fn walk(folder: &ITaskFolder, items: &mut Vec<Item>, errors: &mut Vec<String>) {
        let name = unsafe { folder.Path() }.map(|p| p.to_string()).unwrap_or_default();
        match unsafe { folder.GetTasks(TASK_ENUM_HIDDEN.0) } {
            Ok(tasks) => {
                // Task Scheduler collections count from 1.
                for i in 1..=unsafe { tasks.Count() }.unwrap_or(0) {
                    let read =
                        unsafe { tasks.get_Item(&VARIANT::from(i)) }.and_then(|t| unsafe { Ok((t.Path()?, t.Xml()?)) });
                    match read {
                        Ok((path, xml)) => match task_item(&path.to_string(), &xml.to_string()) {
                            Ok(item) => items.push(item),
                            Err(e) => errors.push(e),
                        },
                        Err(e) => errors.push(format!("{name} #{i}: {e}")),
                    }
                }
            }
            Err(e) => errors.push(format!("{name}: {e}")),
        }
        match unsafe { folder.GetFolders(0) } {
            Ok(folders) => {
                for i in 1..=unsafe { folders.Count() }.unwrap_or(0) {
                    match unsafe { folders.get_Item(&VARIANT::from(i)) } {
                        Ok(sub) => walk(&sub, items, errors),
                        Err(e) => errors.push(format!("{name} folder #{i}: {e}")),
                    }
                }
            }
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAILY: &str = include_str!("../../tests/fixtures/task-daily-updater.xml");
    const LOGON_COM: &str = include_str!("../../tests/fixtures/task-logon-com.xml");

    #[test]
    fn daily_exec_task_is_parsed() {
        let item = task_item(r"\DiscordUpdate", DAILY).unwrap();
        assert_eq!(item.key, r"\discordupdate");
        assert_eq!(item.label, r"\DiscordUpdate");
        assert!(
            item.target.as_deref().unwrap().ends_with(r"\discord\update.exe"),
            "{:?}",
            item.target
        );
        assert_eq!(item.props["triggers"], "Daily");
        assert_eq!(item.props["user"], "S-1-5-18");
        assert_eq!(item.props["run_level"], "HighestAvailable");
        assert_eq!(item.props["enabled"], "true");
        assert_eq!(item.props["args"], "--processStart Discord.exe");
        assert!(!item.props.contains_key("com"));
    }

    #[test]
    fn logon_com_task_has_no_target() {
        let item = task_item(r"\Microsoft\Windows\Shell\FamilySafetyMonitor", LOGON_COM).unwrap();
        assert_eq!(item.target, None);
        assert_eq!(item.props["com"], "{A6BA00FE-40E8-477C-B713-C64A14F18ADB}");
        assert_eq!(item.props["triggers"], "Logon");
        assert_eq!(item.props["enabled"], "false");
        // A group principal stands in when there's no user.
        assert_eq!(item.props["user"], "S-1-5-32-545");
        assert!(!item.props.contains_key("args"));
    }

    #[test]
    fn garbage_xml_is_an_error() {
        assert!(task_item(r"\x", "<Task><Triggers>").is_err());
        assert!(task_item(r"\x", "not xml at all").is_err());
        assert!(task_item(r"\x", "<NotATask/>").is_err());
    }

    #[test]
    fn trigger_kinds_are_named() {
        let xml = r#"<Task xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task"><Triggers>
            <CalendarTrigger><ScheduleByWeek/></CalendarTrigger>
            <CalendarTrigger><ScheduleByMonth/></CalendarTrigger>
            <CalendarTrigger><ScheduleByMonthDayOfWeek/></CalendarTrigger>
            <BootTrigger/><IdleTrigger/><TimeTrigger/><EventTrigger/><BootTrigger/>
            </Triggers><Actions><Exec><Command>rundll32.exe</Command><Arguments>C:\App\x.dll,Run</Arguments></Exec></Actions></Task>"#;
        let item = task_item(r"\t", xml).unwrap();
        assert_eq!(item.props["triggers"], "Weekly,Monthly,Boot,Idle,Time,EventTrigger");
        // A launcher's target is the file it's given.
        assert_eq!(item.target.as_deref(), Some(r"c:\app\x.dll"));
        assert_eq!(item.props["enabled"], "true");
    }
}
