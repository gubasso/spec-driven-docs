//! Machine-facing binary entry point.
//!
//! `main` is the process boundary and nothing else: it emits one structured
//! error envelope and classifies it once. All the work — parsing args,
//! initializing logging, building `AppContext`, dispatching — lives in
//! `run`, which returns `Result` so `?` is available. No business logic
//! here or in `run`; that lives in `commands`, `services`, and `gates`.

use std::process::ExitCode;

use clap::Parser;
use spec_driven_docs::cli::{Cli, Commands};
use spec_driven_docs::context::AppContext;
use spec_driven_docs::error::AppError;
use spec_driven_docs::services::reader;
use spec_driven_docs::{commands, logging, output};

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output::error_envelope(&e);
            ExitCode::from(e.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    logging::init(cli.global.verbose)?;
    let ctx = AppContext::new(cli.global.verbose)?;

    match cli.command {
        Commands::Init(args) => commands::init::run(&ctx, args),
        Commands::Verify(args) => commands::verify::run(&ctx, args),
        Commands::Upgrade(args) => commands::upgrade::run(&ctx, args),
        Commands::Gate(args) => commands::gate::run(&ctx, args),
        Commands::Hooks(args) => commands::hooks::run(&ctx, args),
        Commands::Method(args) => commands::read::run(&ctx, &reader::METHOD, args),
        Commands::Spec(args) => commands::read::run(&ctx, &reader::SPECS, args),
        Commands::Template(args) => commands::read::run(&ctx, &reader::TEMPLATES, args),
        Commands::Skill(args) => commands::skill::run(&ctx, args),
        Commands::Status(args) => commands::status::run(&ctx, args),
        Commands::Doctor(args) => commands::doctor::run(&ctx, &args),
        Commands::Assess(args) => commands::assess::run(&ctx, args),
        Commands::License(args) => commands::license::run(&ctx, args),
        Commands::SelfManifest => commands::self_manifest::run(&ctx),
        Commands::Completions(args) => commands::completions::run(&ctx, args),
        Commands::Man => commands::man::run(&ctx),
    }
}
