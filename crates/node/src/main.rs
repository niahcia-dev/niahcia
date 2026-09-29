use std::env;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "NIAHCIA {VERSION}

Usage:
  niahcia [OPTIONS]

Options:
  -h, --help       Print help
  -V, --version    Print version

Status:
  Pre-alpha reference node bootstrap.
"
    );
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        None => {
            println!("NIAHCIA {VERSION}");
            println!("pre-alpha node bootstrap");
            println!("No network services are implemented yet.");
            ExitCode::SUCCESS
        }
        Some("-h") | Some("--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("-V") | Some("--version") => {
            println!("niahcia {VERSION}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown argument: {other}");
            eprintln!("try 'niahcia --help'");
            ExitCode::from(2)
        }
    }
}
