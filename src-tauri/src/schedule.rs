// Weekly background snapshot: a per-user Task Scheduler task that runs
// `crumbtrail.exe --snapshot` once a week. No admin needed. The task itself is
// the only state, so "enabled" means "the task exists".

use chrono::{Datelike, Duration, Local, NaiveDateTime, TimeZone};
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;

const TASK: &str = "Crumbtrail weekly snapshot";
const NO_WINDOW: u32 = 0x0800_0000;
const FMT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Serialize)]
pub struct Weekly {
    pub enabled: bool,
    pub next_run: Option<i64>,
}

fn schtasks(args: &[&str]) -> Result<String, String> {
    let out = Command::new("schtasks")
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| format!("couldn't run schtasks: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// First weekly run at or after `now`, counting from the trigger's start.
fn next_run(start: NaiveDateTime, now: NaiveDateTime) -> NaiveDateTime {
    if start >= now {
        return start;
    }
    let week = Duration::weeks(1);
    let passed = (now - start).num_seconds() / week.num_seconds() + 1;
    start + week * passed as i32
}

fn start_boundary(xml: &str) -> Option<NaiveDateTime> {
    let open = "<StartBoundary>";
    let s = xml.find(open)? + open.len();
    let e = s + xml[s..].find("</StartBoundary>")?;
    // Boundaries may carry a UTC offset or fractional seconds; the first 19 chars are local time.
    NaiveDateTime::parse_from_str(xml[s..e].get(..19)?, FMT).ok()
}

pub fn status() -> Weekly {
    // A failed query means the task isn't there (or can't be read): either way, not scheduled.
    match schtasks(&["/Query", "/TN", TASK, "/XML"]) {
        Ok(xml) => Weekly {
            enabled: true,
            next_run: start_boundary(&xml).and_then(|start| {
                let next = next_run(start, Local::now().naive_local());
                Local.from_local_datetime(&next).earliest().map(|t| t.timestamp())
            }),
        },
        Err(_) => Weekly {
            enabled: false,
            next_run: None,
        },
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn task_xml(exe: &str, start: NaiveDateTime) -> String {
    let day = match start.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Saves a snapshot of folder sizes on your system drive so Crumbtrail can show what changed. Folder names and sizes only. Turn it off in Crumbtrail: Space, What changed.</Description>
  </RegistrationInfo>
  <Triggers>
    <CalendarTrigger>
      <StartBoundary>{start}</StartBoundary>
      <Enabled>true</Enabled>
      <ScheduleByWeek>
        <DaysOfWeek><{day} /></DaysOfWeek>
        <WeeksInterval>1</WeeksInterval>
      </ScheduleByWeek>
    </CalendarTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <ExecutionTimeLimit>PT1H</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--snapshot</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        start = start.format(FMT),
        exe = xml_escape(exe),
    )
}

/// Schedule the task: same weekday next week, at noon. Missed runs (PC off) catch up at next logon.
pub fn enable() -> Result<Weekly, String> {
    let exe = std::env::current_exe().map_err(|e| format!("couldn't find Crumbtrail's exe: {e}"))?;
    let start = (Local::now().date_naive() + Duration::days(7))
        .and_hms_opt(12, 0, 0)
        .ok_or("bad start time")?;
    let xml = task_xml(&exe.display().to_string(), start);

    // schtasks wants the file as UTF-16 with a BOM.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(|u| u.to_le_bytes()));
    let path = std::env::temp_dir().join(format!("crumbtrail-task-{}.xml", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
    let res = schtasks(&["/Create", "/TN", TASK, "/XML", &path.display().to_string(), "/F"]);
    let _ = std::fs::remove_file(&path);
    res.map_err(|e| format!("Windows wouldn't create the task: {e}"))?;
    Ok(status())
}

pub fn disable() -> Result<Weekly, String> {
    if status().enabled {
        schtasks(&["/Delete", "/TN", TASK, "/F"])
            .map_err(|e| format!("Windows wouldn't remove the task: {e}"))?;
    }
    Ok(status())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, FMT).unwrap()
    }

    #[test]
    fn next_run_is_start_when_in_future() {
        assert_eq!(next_run(t("2026-10-07T12:00:00"), t("2026-10-01T09:00:00")), t("2026-10-07T12:00:00"));
    }

    #[test]
    fn next_run_rolls_forward_whole_weeks() {
        assert_eq!(next_run(t("2026-10-07T12:00:00"), t("2026-10-07T12:00:01")), t("2026-10-14T12:00:00"));
        assert_eq!(next_run(t("2026-10-07T12:00:00"), t("2026-10-30T08:00:00")), t("2026-11-04T12:00:00"));
    }

    #[test]
    fn reads_start_boundary_with_offset() {
        let xml = "<Triggers><StartBoundary>2026-10-07T12:00:00.000-05:00</StartBoundary></Triggers>";
        assert_eq!(start_boundary(xml), Some(t("2026-10-07T12:00:00")));
    }

    #[test]
    fn escapes_exe_path() {
        let xml = task_xml(r"C:\A & B\crumbtrail.exe", t("2026-10-07T12:00:00"));
        assert!(xml.contains(r"C:\A &amp; B\crumbtrail.exe"));
        assert!(xml.contains("<Wednesday />"));
    }
}
