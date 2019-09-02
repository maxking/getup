Creating a new init system
====================================

Init system is the first process that starts up in any operating system. There
are various different existing init systems [systemd][1], [sysvinit][2] for
Unix/Linux systems and [launchd][3] for macOS systems.

Right now, I am not sure what exactly does works on what and the difference
between each of them. While I have read the introductory [blog post about
systemd][4] which sheds some light on responsibilities and design of an init
system.

The purpose of this project is to just learn how init systems work and is
mostly an academic exercise. There is a low possibility for this to become a
prod level thing one day, but I can be convinced otherwise if this becomes
something better than existing systems.

Another motivation to do this is learn Rust, which is the new systems
programming language from Mozilla. So, let's begin.


Setting up the project
--------------------------

Assuming you have `cargo` and `rustc` setup already, we can get started with a
new binary project. I am going to name my project _getup_.

```bash
$ cargo new --bin getup
```

This should setup a new `getup` directory.

Initial Design
-----------------

These are some very basic design ideas that I currently have of the
project. For starters, I want to re-use the systemd configuration files because
it is the easiest way to replace it on a running system since almost all of
them now ship with systemd and the config.

The MVP for this project would include:

- Ability to parse all the systemd configs and startup all the daemons
- Ability to watch, restart, stop and manage daemon processes
- Ability to determine basic dependencies between systemd units


I am not going to work on delayed launch and other fun stuff systemd and
launchd do to improve the boot speeds. The goal right now is to get a
functional init system that is useful enough that I can use it on a simple
system.

Dependencies
--------------

First, we need to be able to parse systemd unit files, so, we need an ini
parser in Rust.

```
# Cargo.toml
[dependencies]
rust-ini = "0.13.0"
```


Strucutres
------------

We will first start will some `struct` objects that we will need in order to
create objects from systemd files.

```rust
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

    after: Option<Arc<Unit>>,
    before: Option<Arc<Unit>>,
    wants: Option<Arc<Unit>>,
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
```

Each of the structures above are meant to parse sections `[unit]`, `[install]`
and `[service]` in a systemd config file.

Although, there is complex relationship between these unit files, we are just
going to put them in a sequence for now. So we also create another structure.

```rust
/// A collection of all the unit files in a system.
struct AllUnits {
    units: Vec<Unit>,
}
```

Parsing Unit files
----------------------

Now, that we have a very basic structure ready, let's start parsing the systemd
unit files. We are going to use [rust-init][5] package to parse the files since
they are simple ini files.

[This blog post][6] by the author of systemd explains in great detail what does
a systemd unit file include and what the use case of each section is. While it
only includes basic information, that should be enough for us to get started.

[6]: http://0pointer.de/blog/projects/systemd-for-admins-3.html

Some small notes from the above blog:

- `ExecStart` is the path to the binary including any command line flags and if
  the daemon uses double-forking, you need to specify `Type=forking` too.

- If daemon doesn't use double-forking, which is actually not needed in case of
  an active process manager, they can just run the main process and use
  `Type=dbus` and a `BusName=bus.name` because that is how systemd infers that
  the process as finished starting up.

- There are some special systemd units `systemd.special(7)` which has special
  meanings for standardization reasons, like `syslog.target` for any
  implementation of _syslog_.

Runlevels and Targets
-------------------------

Older SysV init scripts had a [concept of run-level][7] to serialize the startup of
daemons into logical groups. Of the run levels, some interesting ones are:

- runlevel 1: single user text mode
- runlevel 2: not defined by default, can be user defined
- runlevel 3: multi-user console mode
- runlevel 4: not defined by default, can be user defined
- runlevel 5: graphical mode
- runlevel 6: reboot

[7]: https://www-uxsup.csx.cam.ac.uk/pub/doc/redhat/enterprise3/rhel-rg-en-3/s1-boot-init-shutdown-sysv.html

systemd converts these into what it calls _targets_. There is
`multi-user.target` which is supported to be same as runlevel 3 and then there
is `graphical.target` to represent runlevel 5. I like this better because it is
kind of obvious what these targets mean, without having to lookup the
definition of various runlevels.

Milestone 1
-------------

So, the first step for this tool is going to be a CLI which can be used to
parse a systemd unit file, spawn off a process using it and then keep
monitoring it for crashes. It does not accept any other arguments and there
would be no way to signal it to stop or restart the process without killing the
tool itself.

In rust, you can add commands/binaries by creating a `src/bin/` directory. Our
tool is called `runone` and so we create `src/bin/runone.rs`.

Implementation
-----------------

Now, we need to setup a few things to get to out Milestone. First, we need to
be able to parse the unit file, here is a small example:

```
[Unit]
Description=GNU Mailman
Documentation=man:mailman(1)

[Service]
Type=notify
ExecStart=/home/maxking/.virtualenvs/mm3/bin/master
ExecReload=/home/maxking/.virtualenvs/mm3/bin/mailman restart

[Install]
WantedBy=multi-user.target
```

We implement the following methods for the `Unit` struct:

```rust
impl Unit {
  pub fn from_unitfile(inifile: &str) -> Unit {
    ...
  }
}
```

Once we have the parsing done and we are able to generate the `Unit` struct, we
then need the ability to use it to start a process. We implement `start()`
method in the `Service` struct since it has all the information to start an
monitor a process.

```rust
impl Service {
  /// status returns the current status of this service.
  pub fn status(&self) -> CurrState {
    self.current_state
  }

  /// start boots up a service and sets it's current status, which
  /// is why it needs a mutable reference to the object.
  pub fn start(&mut self) {
     ...
  }
}
```

Finally, we implement a `main()` in `runone.rs` to tie all the pieces
together.


Monitoring
------------

Now, we have a working implementation of a daemon which boots up and starts up
the service by parsing a systemd unit file. Next step is look out for the
process that was just spun off.

So because parents have to take care of their children, Rust's
`std::process:Child` includes a method `try_wait` method which is non-blocking
method which returns the exit status of the process if it exitted and otherwise
just returns a `None` value. We can use this to create a simple infinite loop
in a thread to monitor this process.

```rust
// monitor.rs
pub fn monitor_proc(child: &mut Child) -> Option<ExitStatus> {
    let thirty_millis = time::Duration::from_millis(30);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                println!("Child proc with PID {:?} exitted with status {:?}", child.id(), status);
                return Some(status);
            },
            Ok(None) => {
                // This really means that the process hasn't exitted yet. In
                // which case, we don't do anything.
                thread::sleep(thirty_millis);
                print!(".");
                io::stdout().flush().unwrap();
                continue
            },
            Err(e) => {
                println!("Failed to wait for the child process: {:?}", e);
                return None;
            }
        }
    }
}
```


A few things about the code above, it takes in a `std::process::Child` and
returns an `Option<ExitStatus>`. It returns the exit status if the processes
exitted and returns None in cases where it wasn't able to wait for the process
for whatever reason.

We print out a single `.` for the feedback purposes to make sure it is actually
working, I get impatient :)


Stopping Process
--------------------

So, now we have an infinite loop monitoring our process and can print out
information when the processes exits. This is neat for when we would need to
restart the process. But, before that, we need to be able to forcefully stop a
process.

In Unix, processes uses various kinds of signals (`man kill`) to signal child
processes to perform actions. It is a form of Inter Process Communication (IPC)
to allow processes to talk to each other.

There are many signals defined in modern Linux systems that can be found by
looking at the output of `kill -l` command which lists all the
signals. However, the interesting one for us right now is `SIGTERM` and
`SIGKILL`.

- `SIGTERM` basically means asking the child process to gracefully
  exit. Usually, the init and supervisor process waits for a certain time after
  this signal is sent to give the child process time to clean up.

- `SIGKILL` will forcefully kill the process. This is used by if the child
  process didn't gracefully exit by the end of timeout given by the
  supervisor/parent process.

Since child processes are expected to perform some actions based on the type of
the signal, processes usually register signal handlers so they can do the
required action whenever a signal arrives. A process can handle all the signals
except `SIGKILL`, which never really is delivered to the process and instead
kills it.

So, let's try to write a signal handler for our supervisor process which will
ask the child process to terminate and then kill it if you press `Ctrl+C` on
the terminal. On Unix systems, a `Ctrl+C` on terminal when a process is running
sends them a `SIGINT` signal. We can attach our handler to this signal using a
Rust crate `ctrlc` which is meant for this exact purpose.


Signal Handling in Rust
-----------------------------

```rust
// runone.rs#main()
    let shared = Arc::new(AtomicBool::new(false));
    let shared_clone = shared.clone();

    ctrlc::set_handler(move || {
        // If the user wants to exit, raise the flag to signal the running
        // thread to kill the child process.
        shared.store(true, Ordering::Relaxed);
    });
```

Note that `ctrlc` is a very specific crate which lets us handle *only*
`SIGINT`, but for now, we only need that. In future, there would be a need for
a more general mechanism which allows handling *any* signal.

What does out signal handler do? It basically just raises a flag to let the
infinite loop know that we got an interrupt and it should try to stop the
process. Now, because in Rust one object can't be reference from two different
threads, we use the `Arc` type, which stands for Atomic Reference Count. This
ads ref counting on the contained object when you do a `.clone()` and returns a
new object that you can use to access the same value from different
thread. Neat.

Out value is `AtomicBool`, which is a simple boolean datatype with Atomic
properties and is thread safe. We need an Atomic type because we are sharing
the reference in two threads and don't want to shoot ourselves in the foot by
causing a race condition. For now, we know that the monitoring thread only
needs to read this flag.


```rust
// monitor.rs#monitor_proc()

  loop {
        let ten_sec = time::Duration::from_millis(10000);

        if shared.load(Ordering::Relaxed) {
            // Shared is a flag for parent process to signal this process to
            // terminate.
            println!("Killing the child process.");
            service.send_term();
            thread::sleep(ten_sec);
            service.kill();
        }
		
		match child.try_wait() {
		    ...
		}
   }
```

We passed the `shared_clone` Arc object to the monitoring thread and it checks
for the flag in every loop to make sure if it needs to terminal the process. 

Rust's child abstraction, `std::process::Child` doesn't allow sending random
signals to a process in a platform independent way yet, so I don't know how to
simply send a `SIGTERM` to the child yet, but it does include `.kill()` method
which will send a `SIGKILL`. For now, we will just noop in the `send_term()`
and come back to it later. I know, we are being really really bad parents.
