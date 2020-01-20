use ini::Ini;
use lazy_static::lazy_static;
use nix::errno::Errno::{EINVAL, EPERM, ESRCH};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use nix::Error::Sys;
use serde::Serialize;
use serde_json;
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::string::ToString;
use std::sync::Arc;
use std::sync::Mutex;
use std::{thread, time};

#[derive(Debug, Serialize)]
pub struct Install {
    wanted_by: Option<String>,
    alias: Option<String>,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum RestartMethod {
    OnFailure,
    Always,
    Never,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum KillModeEnum {
    Process,
    All,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub enum CurrState {
    Stopping,
    Stopped,
    Starting,
    Running,
    Failed,
    Restarting,
}

/// A collection of all the unit files in a system.
#[derive(Debug, Serialize)]
pub struct AllUnits {
    units: Vec<Unit>,
}

impl AllUnits {
    pub fn new() -> AllUnits {
        AllUnits { units: vec![] }
    }

    pub fn add_unit(&mut self, u: Unit) {
        self.units.push(u)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Unit> {
        // Given the name of a service, return if it exists
        self.units.iter().find(|&x| x.path.ends_with(name))
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

/// A Unit is a systemd unit which could contain a Service. It also includes
/// Install which can be used to determine how this service is Installed on a
/// system.
#[derive(Debug, Serialize)]
pub struct Unit {
    /// Path to the systemd config file on the host from where it was read.
    pub path: String,
    /// Description of the Unit.
    pub description: String,
    /// Man pages/documentation for the Unit.
    pub documentation: Option<String>,

    /// Associated Service.
    pub service: Arc<Mutex<Service>>,

    /// How to install this Unit.
    pub install: Install,

    // We use Option and Arc here because these values can be None and we need
    // to Read these values through multiple threads and might need sharing,
    // because of which, we need some sort of reference counting structure.
    /// Start this unit file after the service file for after is started.
    after: Option<Arc<Unit>>,

    before: Option<Arc<Unit>>,

    wants: Option<Arc<Unit>>,
}

impl Unit {
    pub fn from_unitfile(inifile: &Path) -> Unit {
        let conf = Ini::load_from_file(inifile.to_str().unwrap()).unwrap();
        let unit =
            conf.section(Some("Unit".to_owned())).expect("failed to get section: Unit");
        let service = conf
            .section(Some("Service".to_owned()))
            .expect("failed to get section: Service");
        let install = conf
            .section(Some("Install".to_owned()))
            .expect("failed to get section: Install");

        let _documentation = None;
        if let Some(desc) = unit.get("Documentation") {
            let _documentation = Some(desc.to_string());
        };

        let _exec_reload = None;
        if let Some(exec) = service.get("ExecReload") {
            let _exec_reload = Some(exec.to_string());
        }

        let atype = None;
        if let Some(atp) = service.get("Type") {
            let _atype = Some(atp.to_string());
        }

        Unit {
            path: inifile.to_str().unwrap().to_string(),
            description: unit
                .get("Description")
                .expect("failed to get Description from Unit")
                .to_string(),
            documentation: _documentation,
            service: Arc::new(Mutex::new(Service {
                service_type: atype,
                exec_start: service
                    .get("ExecStart")
                    .expect("failed to get ExecStart from Service")
                    .to_string(),
                exec_reload: _exec_reload,
                restart: None,
                no_new_privs: None,
                capability_bounding_set: None,
                current_state: CurrState::Stopped,
                child: None,
                exit_status: None,
                restart_policy: RestartMethod::OnFailure,
            })),
            install: Install {
                wanted_by: None,
                alias: match install.get("Alias") {
                    Some(value) => Some(value.to_string()),
                    None => None,
                },
            },
            after: None,
            before: None,
            wants: None,
        }
    }
}

/// Service file which includes information on how to start, stop, kill or
/// reload a daemon service.
#[derive(Debug, Serialize)]
pub struct Service {
    /// There are different types of Services, for now, all I know is that they
    /// are different kinds of them.
    pub service_type: Option<String>,
    /// Command to start a daemon, can be a command with arguments, delimited
    /// by empty whitespace.
    pub exec_start: String,
    /// Command to reload the configuration for the daemon.
    pub exec_reload: Option<String>,
    /// Command to restart the service.
    pub restart: Option<RestartMethod>,
    /// Limit the capabilities of the child spawned process.
    pub capability_bounding_set: Option<String>,
    /// Disable the daemon process from gaining any new privileges.
    pub no_new_privs: Option<bool>,
    /// What is the current state of this service.
    pub current_state: CurrState,

    /// The handle to the child process.
    #[serde(skip_serializing)]
    child: Option<Child>,

    pub restart_policy: RestartMethod,

    #[serde(skip_serializing)]
    pub exit_status: Option<ExitStatus>,
}

impl Service {
    pub fn status(&self) -> CurrState {
        self.current_state
    }

    pub fn child_id(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.as_mut().unwrap().try_wait()
    }

    pub fn start(&mut self) {
        let exec_args: Vec<&str> = self.exec_start.split_whitespace().collect();
        let mut cmd = Command::new(exec_args[0]);
        self.current_state = CurrState::Starting;

        // We need to set the args if there are any.
        if exec_args.len() > 1 {
            cmd.args(&exec_args[1..]);
        }
        cmd.stdout(Stdio::piped());

        self.child =
            Some(cmd.spawn().expect(&format!(
                "failed to spawn child process for {:?}",
                exec_args[0]
            )));
        self.current_state = CurrState::Running;
    }

    /// Send SIGTERM to the process and if it does not exit after a timeout,
    /// send a SIGKILL.
    pub fn stop(&mut self) {
        // Reference for this implementation:
        // https://gist.github.com/spwitt/2f8f116fffeb0f3135df963d4bdf0637

        self.current_state = CurrState::Stopping;
        // Graceful exit timeout is 10 seconds by default./
        // TODO: Make this configurable per-service, like systemd.
        let wait_duration = time::Duration::new(10, 0);
        let pid = Pid::from_raw(self.child.as_ref().unwrap().id() as i32);
        match kill(pid, Signal::SIGINT) {
            Ok(()) => {
                let expire = time::Instant::now() + wait_duration;
                while let Ok(None) = self.try_wait() {
                    if time::Instant::now() > expire {
                        break;
                    }
                    thread::sleep(wait_duration / 10);
                }
                if let Ok(None) = self.try_wait() {
                    self.kill()
                }
            }
            Err(Sys(EINVAL)) => {
                println!("Invalid signal. Killing process");
                self.kill()
            }
            Err(Sys(EPERM)) => {
                println!("Insufficient permissions to signal process {}", pid);
            }
            Err(Sys(ESRCH)) => {
                println!("Process identified by {} does not exist", pid);
            }
            Err(e) => println!("Unexpected error {}", e),
        }
        self.current_state = CurrState::Stopped;
    }

    pub fn kill(&mut self) {
        println!("Trying to kill service started by: {:?}", self.exec_start);
        self.child.as_mut().unwrap().kill().expect("Failed to kill chill process");
        println!("Killed child service started by: {:?}", self.exec_start);
    }

    pub fn reload() {}

    pub fn restart() {}
}

// A global instance of AllUnits to store the loaded values at runtime.
lazy_static! {
    pub static ref ALL_UNITS: Arc<Mutex<AllUnits>> =
        Arc::new(Mutex::new(AllUnits::new()));
    // pub static ref BASE_PATH: &str = "".to_string();
}

// Reload the server looking for any new services.
pub fn reload_server() {}
