use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::Child;
use std::process::Command;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::JobObjects::AssignProcessToJobObject;
use windows::Win32::System::JobObjects::CreateJobObjectW;
use windows::Win32::System::JobObjects::JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
use windows::Win32::System::JobObjects::JOBOBJECT_EXTENDED_LIMIT_INFORMATION;
use windows::Win32::System::JobObjects::JobObjectExtendedLimitInformation;
use windows::Win32::System::JobObjects::SetInformationJobObject;
use windows::Win32::System::Threading::DETACHED_PROCESS;

pub trait SpawnJobExt {
    fn spawn_job(self) -> eyre::Result<Child>;
}
impl SpawnJobExt for &mut Command {
    fn spawn_job(self) -> eyre::Result<Child> {
        spawn_job(self)
    }
}

/// Spawn the command as a child process in a job.
/// This ties the lifetime of the child process to the lifetime of this process.
/// When this process exits, the child process will also be terminated.
/// The child process is detached from the console, so it won't show a console window.
/// See [`crate::console`] for ways to attach to the console if needed.
pub fn spawn_job(command: &mut Command) -> eyre::Result<Child> {
    // Create a job object that kills processes when the handle is closed
    let job_handle = unsafe { CreateJobObjectW(None, None)? };
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )?;
    }

    // Spawn the process
    let mut child = command.creation_flags(DETACHED_PROCESS.0).spawn()?;

    // Attach child process to the job
    attach_to_job(job_handle, &mut child)?;

    Ok(child)
}

fn attach_to_job(job_handle: HANDLE, child: &mut Child) -> eyre::Result<()> {
    let proc_handle = HANDLE(child.as_raw_handle());
    unsafe {
        AssignProcessToJobObject(job_handle, proc_handle)?;
    }

    // Leak the job handle so it stays valid until this process exits,
    // ensuring the GUI is killed if the tray process terminates.
    Box::leak(Box::new(job_handle));

    Ok(())
}
