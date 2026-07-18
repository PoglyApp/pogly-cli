#![recursion_limit = "512"]

mod api;
mod cli;
mod commands;
mod config;
mod help;
mod output;
mod paths;
mod update;
mod versionstore;

use clap::FromArgMatches;

fn main() {
    update::passive_check();
    let command = <cli::Cli as clap::CommandFactory>::command()
        .before_help(help::banner())
        .help_template(help::HELP_TEMPLATE);
    let matches = command.get_matches();
    let parsed = match cli::Cli::from_arg_matches(&matches) {
        Ok(parsed) => parsed,
        Err(e) => e.exit(),
    };
    if let Err(error) = commands::run(parsed) {
        eprintln!("error: {error:#}");
        let api_failure = error.chain().any(|cause| {
            cause.downcast_ref::<api::client::ApiError>().is_some()
                || cause.downcast_ref::<ureq::Error>().is_some()
        });
        std::process::exit(if api_failure { 1 } else { 2 });
    }
}
