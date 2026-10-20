use std::io::{self, Write};

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let quiet = args.first().is_some_and(|arg| arg == "complete");
    let code = match git_autocomplete::cli::run(&args) {
        Ok(output) => match io::stdout().write_all(output.as_bytes()) {
            Ok(()) => 0,
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => 0,
            Err(err) => {
                if !quiet {
                    eprintln!("git-autocomplete: {err}");
                }
                1
            }
        },
        Err(err) => {
            if !quiet {
                eprintln!("git-autocomplete: {err}");
            }
            2
        }
    };
    std::process::exit(code);
}
