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
    // #[options(help = "Register a speller with the system")]
    // Register(RegisterArgs),

    // #[options(help = "Deregister a speller from the system")]
    // Deregister(DeregisterArgs),
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

fn setup_logger() {
    let log_path = pathos::system::app_log_dir("WinDivvun");
    match std::fs::create_dir_all(&log_path) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            eprintln!("LOGGING IS DISABLED!");
            return;
        }
    }

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {:<5} {}] {}",
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file(log_path.join("spelli.log")).unwrap())
        .apply()
        .unwrap();
}

fn main() {
    let args = Args::parse_args_default_or_exit();
    setup_logger();

    let command = match args.command {
        Some(v) => v,
        None => {
            eprintln!("Missing required subcommand.");
            std::process::exit(1);
        }
    };

    match command {
        // Command::Register(args) => register::register(args),
        // Command::Deregister(args) => deregister::deregister(args),
        Command::Refresh(_args) => {
            refresh::refresh().unwrap();
        }
        Command::List(_args) => {
            list::list();
        }
        Command::Nuke(_args) => {
            crate::reg::nuke_key().unwrap();
        }
    }
}
