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

We will take actions based on the `RestartPolicy` later, for now, we just
lookout for the process and report the state to the user.


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


Restart Policy
-----------------

Among other responsibilities of an init system, it also needs to restart a
service when it dies. systemd allows users to specify a `RestartMethod` which
defines the policy on when should the process be restarted. The possible values
are:

- OnFailure
- Always
- Never

In Rust, we define these values using an Enum:

```rust
#[derive(Debug, Copy, Clone)]
pub enum RestartMethod {
    OnFailure,
    Always,
    Never,
}
```

Sharing references to Object across threads
------------------------------------------------------

Rust is fun. It provides guarantees for memory safety for compiled programs
without a runtime or GC. It is nice, but honestly quite frustating to use at
time.

This problem came up when trying to implement the Restart Policy for our toy
init system. We spin off the monitoring of process is a separate thread so we
can continue with doing other stuff. In order to do that, we *move* the `struct
Service` to the thread, which is one the ways to use the `thread::spawn` API
with a closure definition:

```rust
// rustone.rs#main()

    let mon_thread = thread::spawn(move || {
        monitor::monitor_proc(&mut unit.service, &shared_clone);
    });
```

We need to share a mutable reference because the monitoring process sets the
correct `exit_status` and `current_state` of the `Service`.

```rust
// monitor.rs#monitor_proc()

            Ok(Some(status)) => {
                println!(
                    "Child proc with PID {:?} exitted with status {:?}",
                    service.child_id(),
                    status
                );
                service.current_state = CurrState::Stopped;
                service.exit_status = Some(status);
            }
```

To handle the restart, what I initially thought, was I'll just use the
`unit.service.exit_status` to determine how the process exitted and then
restart based on the policy. Here is a simple example, which doesn't do much
except print in the parent thread how the child exitted:

```rust
// rustone.rs#main()

    let mon_thread = thread::spawn(move || {
        monitor::monitor_proc(&mut unit.service, &shared_clone);
    });

    let _ = mon_thread.join();

    println!("Process exitted with status: {:?}", unit.service.exit_status);
```


But guess what? Rust won't let me compile this program:

```

error[E0382]: borrow of moved value: `unit`
  --> src/bin/runone.rs:46:51
   |
22 |     let mut unit = units::Unit::from_unitfile(&args[1]);
   |         -------- move occurs because `unit` has type `getup::units::Unit`, which does not implement the `Copy` trait
...
40 |     let mon_thread = thread::spawn(move || {
   |                                    ------- value moved into closure here
41 |         monitor::monitor_proc(&mut unit.service, &shared_clone);
   |                                    ---- variable moved due to use in closure
...
46 |     println!("Process exitted with status: {:?}", unit.service.exit_status);
   |                                                   ^^^^^^^^^^^^^^^^^^^^^^^^ value borrowed here after move

error: aborting due to previous error
```

What is going on here? Well, like we said, the function `thread::spawn` moves
the `unit.service` into the new thread and once it is done with it, no one else
can actually do anything with it since it has died with the monitoring
thread. There is no simple way to return the reference back, like you could do
with a function call.

Note that, given how the code is written, it may seem like it is thread safe. I
spawn a thread, I move a reference to it, I wait for it to finish and then
finally I want to re-gain control of the object. There are clearly no two
threads trying to mutate or mutate + read in separate threads to ever cause a
race-condition.

> spawn launches independent threads. Rust has no way of knowing how long the
> child thread will run, so it assumes the worst: it assumes the child thread may
> keep running even after the parent thread has finished and all values in the
> parent thread are gone. Obviously, if the child thread is going to last that
> long, the closure it’s running needs to last that long too. But this closure
> has a bounded lifetime: it depends on the reference glossary, and references
> don’t last forever.
>
> Note that Rust is right to reject this code! The way we’ve written this
> function, it is possible for one thread to hit an I/O error, causing
> process_files_in_parallel to bail out before the other threads are
> finished. Child threads could end up trying to use the glossary after the main
> thread has freed it. It would be a race—with undefined behavior as the prize,
> if the main thread should win. Rust can’t allow this.
>
>                - Chater 19, Programming Rust by Jason Orendorff, Jim Blandy


I spent some time reading more about this behavior and found this in the
[Programming Rust by Jason Orendorff, Jim Blandy][prog_rust] to figure out why
doesn't rust allow this. It moves on to explain how to use `Arc` to share
references across threads in a thread safe manner with reference counting so it
doesn't get freed while there are references to the object. There also exists
`Rc` type which cab be used for reference counting in a single thread, which
has lower overhead than `Arc` if you don't need to pass along the reference
across threads.

[prog_rust]: https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1491927283/

To achieve this, we changed `service` to be an Arc object, with a Mutex, so
that it is safe to mutate from two threads, with proper locking of course:

```rust
// units.rs

pub struct Service {
    ...
	service: Arc<Mutex<Service>>,
	...
}
```

And then we can pass a reference-counted reference to `service` to the
monitoring thread and it can safely be dropped when the thread ends:

```rust
// runone.rs#main()

    let service_clone = unit.service.clone();

    let mon_thread = thread::spawn(move || {
        monitor::monitor_proc(&service_clone, &shared_clone);
    });

    let _ = mon_thread.join();

    println!("Process exitted with status: {:?}", unit.service.lock().unwrap().exit_status);
```

And yay! This compiles and let me use `unit.service` after having moved it to
the monitoring thread.

Restart Policy: Back
-------------------------

Now, let's try to solve our restart-policy problems. We will start simple,
assume that all process want to be restarted after they die. We can hook up the
actual policy later.

```rust
//runone.rs#main()

    loop {
        let service_clone = unit.service.clone();
        let shared_shared_clone = shared.clone();

        let mon_thread = thread::spawn(move | | {
            monitor::monitor_proc(&service_clone, &shared_shared_clone);
        });

        let _ = mon_thread.join().expect("Failed to join the threads");
        unit.service.lock().unwrap().start();
    }

```

So, this is simple, we basically loop around waiting for the child process to
exit and just restart it. That is all.

One problem with this approach of restarting the process always is that there
is really no way to exit this infinite loop, we will just keep spinning and
spinning. Even when we use the signal handler we used above to signal the child
process to exit, we still end up restarting it.

But let's make this slightly better than it is right now:

```rust
// runone.rs#main()

    loop {
        let service_clone = unit.service.clone();
        let shared_shared_clone = shared.clone();

        let mon_thread = thread::spawn(move | | {
            monitor::monitor_proc(&service_clone, &shared_shared_clone);
        });

        let _ = mon_thread.join().expect("Failed to join the threads");

        let mut unlocked_service = unit.service.lock().unwrap();
        match unlocked_service.restart_policy {
            RestartMethod::Never => break,
            RestartMethod::Always => {
                println!("Restart policy is RestartMethod::Always...");
                unlocked_service.start();
            },
            RestartMethod::OnFailure => {
                println!("Restart policy is Restart::OnFailure...");
                if unlocked_service.exit_status.unwrap().success() {
                    unlocked_service.start();
                } else {
                    println!("Exitted with exit code 0, so not going to restart.");
                    break;
                }
            },
        }
    }
```

This is a slightly better version, which checks for the restart policy, and
does not restart the process when the policy is `Never`. Do however note that
this still doesn't solve the problem of, what happens when I manually try to
stop a process and it doesn't get restarted, even if it has a `restart_policy`
that allows it to be started. Just let me kill the child process please?!


Maybe, third time is the charm! So, it is possible to determine if a process
was terminated by a signal or if it died of natural causes. We can use this
information to check if the process crashed, or if it was someone (including,
us) who stopped the process:

```rust
// rustone.rs# main()

        match unlocked_service.restart_policy {
            RestartMethod::Never => break,
            RestartMethod::Always => {
                println!("Restart policy is RestartMethod::Always...");
                unlocked_service.start();
            }
            RestartMethod::OnFailure => {
                println!("Restart policy is Restart::OnFailure...");
                match unlocked_service.exit_status.unwrap().code() {
                    Some(code) => {
                        if code != 0 {
                            unlocked_service.start();
                        } else {
                            println!("Exitted with exit code 0, so not going to restart.");
                            break;
                        }
                    }
                    None => {
                        println!(
                            "Exitted with signal: {:?}, so not going to restart.",
                            unlocked_service.exit_status.unwrap().signal().unwrap()
                        );
                        break;
                    }
                }
            }
        }

```
