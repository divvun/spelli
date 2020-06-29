mod deregister;
mod list;
mod refresh;
mod reg;
mod register;

use gumdrop::Options;

#[derive(Debug, Options)]
struct Args {
    #[options(help = "show usage help")]
    help: bool,

    #[options(command)]
    command: Option<Command>,
}

#[derive(Debug, Options)]
enum Command {
    #[options(help = "Register a speller with the system")]
    Register(RegisterArgs),

    #[options(help = "Deregister a speller from the system")]
    Deregister(DeregisterArgs),

    #[options(help = "Refresh registry keys for registered spellers")]
    Refresh(RefreshArgs),

    #[options(help = "List registered spellers")]
    List(ListArgs),

    #[options(help = "Delete all registered spellers")]
    Nuke(NukeArgs),
}

#[derive(Debug, Options)]
struct RegisterArgs {
    #[options(help = "show usage help")]
    help: bool,

    #[options(required, help = "BCP 47 language tag")]
    tag: String,

    #[options(required, help = "Path to ZHFST file to register")]
    path: std::path::PathBuf,
}

#[derive(Debug, Options)]
struct DeregisterArgs {
    #[options(help = "show usage help")]
    help: bool,

    #[options(required, help = "BCP 47 language tag")]
    tag: String,
}

#[derive(Debug, Options)]
struct RefreshArgs {
    #[options(help = "show usage help")]
    help: bool,
}

#[derive(Debug, Options)]
struct ListArgs {
    #[options(help = "show usage help")]
    help: bool,
}

#[derive(Debug, Options)]
struct NukeArgs {
    #[options(help = "show usage help")]
    help: bool,
}

fn main() {
    let args = Args::parse_args_default_or_exit();

    let env = env_logger::Env::default().default_filter_or("debug");
    env_logger::from_env(env).init();

    let command = match args.command {
        Some(v) => v,
        None => {
            eprintln!("Missing required subcommand.");
            std::process::exit(1);
        }
    };

    let result = match command {
        Command::Register(args) => register::register(args),
        Command::Deregister(args) => deregister::deregister(args),
        Command::Refresh(_args) => {
            refresh::refresh();
            Ok(())
        }
        Command::List(_args) => {
            list::list();
            Ok(())
        }
        Command::Nuke(_args) => {
            use std::io::{self, Write};

            print!("Are you sure you want to clear the speller registry? [y/N]> ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_ascii_lowercase() == "y" {
                crate::reg::nuke_key().unwrap();
                return;
            } else {
                eprintln!("Aborted.");
                std::process::exit(1);
            }
        }
    };

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }
}
