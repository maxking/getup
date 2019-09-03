use ini::Ini;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::Mutex;

/// A collection of all the unit files in a system.
pub struct AllUnits {
    units: Vec<Unit>,
}

/// A Unit is a systemd unit which could contain a Service. It also includes
/// Install which can be used to determine how this service is Installed on a
/// system.
#[derive(Debug)]
pub struct Unit {
    /// Path to the systemd config file on the host from where it was read.
    pub path: String,
    /// Description of the Unit.
    pub description: String,
    /// Man pages/documentation for the Unit.
    pub documentation: String,
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
    pub fn from_unitfile(inifile: &str) -> Unit {
        let conf = Ini::load_from_file(inifile).unwrap();
        let unit =
            conf.section(Some("Unit".to_owned())).expect("failed to get section: Unit");
        let service = conf
            .section(Some("Service".to_owned()))
            .expect("failed to get section: Service");
        let install = conf
            .section(Some("Install".to_owned()))
            .expect("failed to get section: Install");

        Unit {
            path: inifile.to_string(),
            description: unit
                .get("Description")
                .expect("failed to get Description from Unit")
                .to_string(),
            documentation: unit
                .get("Documentation")
                .expect("failed to get Documentation from Unit")
                .to_string(),
            service: Arc::new(Mutex::new(Service {
                service_type: service
                    .get("Type")
                    .expect("failed to get Type from Service")
                    .to_string(),
                exec_start: service
                    .get("ExecStart")
                    .expect("failed to get ExecStart from Service")
                    .to_string(),
                exec_reload: Some(
                    service
                        .get("ExecReload")
                        .expect("failed to get ExecReload from Service")
                        .to_string(),
                ),
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
#[derive(Debug)]
pub struct Service {
    /// There are different types of Services, for now, all I know is that they
    /// are different kinds of them.
    pub service_type: String,
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
    child: Option<Child>,

    pub restart_policy: RestartMethod,
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
    }

    pub fn send_term(&mut self) {}

    pub fn kill(&mut self) {
        println!("Trying to kill service started by: {:?}", self.exec_start);
        self.child.as_mut().unwrap().kill();
        println!("Killed child service started by: {:?}", self.exec_start);
    }

    pub fn reload() {}

    pub fn restart() {}
}

#[derive(Debug)]
pub struct Install {
    wanted_by: Option<String>,
    alias: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum RestartMethod {
    OnFailure,
    Always,
    Never,
}

#[derive(Debug, Copy, Clone)]
pub enum KillModeEnum {
    Process,
    All,
}

#[derive(Copy, Clone, Debug)]
pub enum CurrState {
    Stopped,
    Starting,
    Running,
    Failed,
    Restarting,
}
