
async fn reap_till(till: i32) -> Result<(), Pid1Error> {
    let mut status = 0;
    let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())?;

    while stream.recv().await.is_some() {
        loop {
            let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
            if pid == till {
                return Ok(());
            } else if pid <= 0 {
                break;
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum Pid1Error {
    IOError(std::io::Error),
    NoCommandGiven,
    ChildPidTooBig(u32, std::num::TryFromIntError),
}

impl std::convert::From<std::io::Error> for Pid1Error {
    fn from(e: std::io::Error) -> Self {
        Pid1Error::IOError(e)
    }
}

#[tokio::main]
async fn main() -> Result<(), Pid1Error> {
    let (cmd, args) = get_command()?;
    let child = std::process::Command::new(cmd).args(args).spawn()?.id();

    use std::convert::TryInto;
    let child: libc::pid_t = match child.try_into() {
        Ok(x) => x,
        Err(e) => return Err(Pid1Error::ChildPidTooBig(child, e)),
    };

    let interrupt_child = move || {
        unsafe {
            libc::kill(child, libc::SIGINT); // ignoring errors
        }
    };
    let sigid: signal_hook::SigId =
        unsafe { signal_hook::register(signal_hook::SIGINT, interrupt_child)? };

    reap_till(child).await?;

    signal_hook::unregister(sigid);
    Ok(())
}

fn get_command() -> Result<(String, Vec<String>), Pid1Error> {
    let mut args = std::env::args();
    let _me = args.next();
    match args.next() {
        None => Err(Pid1Error::NoCommandGiven),
        Some(cmd) => Ok((cmd, args.collect())),
    }
}
