===================================
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
```

Each of the structures above are meant to parse sections `[unit]`, `[install]`
and `[service]` in a systemd config file.

