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
