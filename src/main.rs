use ini::Ini;

/// A Unit is a systemd unit which could contain a Service. It also includes
/// Install which can be used to determine how this service is Installed on a
/// system.
struct Unit {
    /// Path to the systemd config file on the host from where it was read.
    Path: String,
    /// Description of the Unit.
    Description: String,
    /// Man pages/documentation for the Unit.
    Documentation: String,
    /// Associated Service.
    Service: Service,
    /// How to install this Unit.
    Install: Install,

    After: Unit,
    Before: Unit,
    Wants: Unit,
}

/// Service file which includes information on how to start, stop, kill or
/// reload a daemon service.
struct Service {
    /// There are different types of Services, for now, all I know is that they
    /// are different kinds of them.
    Type: String,
    /// Command to start a daemon, can be a command with arguments, delimited
    /// by empty whitespace.
    ExecStart: String,
    /// Command to reload the configuration for the daemon.
    ExecReload: String,
    /// Command to restart the service.
    Restart: RestartMethod,
    /// Limit the capabilities of the child spawned process.
    CapabilityBoundingSet: String,
    /// Disable the daemon process from gaining any new privileges.
    NoNewPrivs: bool,

    MemoryDenyWriteExecute: bool,
    KillMode: KillModeEnum,
}

struct Install {
    WantedBy: String,
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
