use std::process::ExitCode;

fn main() -> ExitCode {
    blueos_service::run(blueos_example::NAME, env!("CARGO_PKG_VERSION"), blueos_example::serve)
}
