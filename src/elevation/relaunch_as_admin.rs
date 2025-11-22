use crate::elevation::ElevatedChildProcess;
use crate::elevation::run_as_admin;
use crate::invocation::SameInvocationSameConsole;

/// Relaunches the current executable with administrative privileges, preserving arguments and console.
pub fn relaunch_as_admin() -> eyre::Result<ElevatedChildProcess> {
    run_as_admin(&SameInvocationSameConsole)
}
