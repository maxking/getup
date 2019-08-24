use ini::Ini;
use std::sync::Arc;

/// A collection of all the unit files in a system.
struct AllUnits {
    units: Vec<Unit>,
}

/// A Unit is a systemd unit which could contain a Service. It also includes
/// Install which can be used to determine how this service is Installed on a
/// system.
#[derive(Debug)]
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

    // We use Option and Arc here because these values can be None and we need
    // to Read these values through multiple threads and might need sharing,
    // because of which, we need some sort of reference counting structure.
    /// Start this unit file after the service file for after is started.
    after: Option<Arc<Unit>>,
    before: Option<Arc<Unit>>,
    wants: Option<Arc<Unit>>,
}

/// Service file which includes information on how to start, stop, kill or
/// reload a daemon service.
#[derive(Debug)]
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
    /// What is the current state of this service.
    current_state: CurrState,

}

impl Service {
    pub fn status() {}

    pub fn start() {}

    pub fn stop() {}

    pub fn reload() {}

    pub fn restart() {}
}

#[derive(Debug)]
struct Install {
    wanted_by: String,
}

#[derive(Debug)]
enum RestartMethod {
    OnFailure,
    Always,
    Never,
}

#[derive(Debug)]
enum KillModeEnum {
    Process,
    All,
}

#[derive(Debug)]
enum CurrState {
    Stopped,
    Starting,
    Running,
    Failed,
    Restarting,
}


fn main() {
    println!("Hello, world!");
}
