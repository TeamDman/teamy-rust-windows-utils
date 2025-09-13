use crate::{elevation::{run_as_admin, ElevatedChildProcess}, invocation::SameInvocationSameConsole};

/// Relaunches the current executable with administrative privileges, preserving arguments and console.
pub fn relaunch_as_admin() -> eyre::Result<ElevatedChildProcess> {
    run_as_admin(&SameInvocationSameConsole)
}
