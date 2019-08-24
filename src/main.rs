use ini::Ini;
use std::sync::Arc;

/// A collection of all the unit files in a system.
struct AllUnits {
    units: Vec<Unit>,
}



/// A Unit is a systemd unit which could contain a Service. It also includes
/// Install which can be used to determine how this service is Installed on a
/// system.
struct Unit {
    /// Path to the systemd config file on the host from where it was read.
    path: String,
    /// Description of the Unit.
    description: String,
    /// Man pages/documentation for the Unit.
    documentation: String,
    /// Associated Service.
    service: Service,
    /// How to install this Unit.
    install: Install,

    after: Arc<Unit>,
    before: Arc<Unit>,
    wants: Arc<Unit>,
}

/// Service file which includes information on how to start, stop, kill or
/// reload a daemon service.
struct Service {
    /// There are different types of Services, for now, all I know is that they
    /// are different kinds of them.
    service_type: String,
    /// Command to start a daemon, can be a command with arguments, delimited
    /// by empty whitespace.
    exec_start: String,
    /// Command to reload the configuration for the daemon.
    exec_reload: String,
    /// Command to restart the service.
    restart: RestartMethod,
    /// Limit the capabilities of the child spawned process.
    capability_bounding_set: String,
    /// Disable the daemon process from gaining any new privileges.
    no_new_privs: bool,

    memory_deny_write_execute: bool,
    kill_mode: KillModeEnum,
}

struct Install {
    wanted_by: String,
}

enum RestartMethod {
    OnFailure,
    Always,
    Never,
}

enum KillModeEnum {
    Process,
    All,
}

fn main() {
    println!("Hello, world!");
}
