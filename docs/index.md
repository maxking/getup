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
