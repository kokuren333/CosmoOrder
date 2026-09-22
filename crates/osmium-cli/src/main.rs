//! `osmium` binary entry point.

fn main() -> std::process::ExitCode {
    let exit = osmium_cli::run(std::env::args_os().collect());
    std::process::ExitCode::from(exit.code())
}
