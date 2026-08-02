use clap::Parser;

use complai::cli::{Cli, Commands};

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Skill { command } => complai::skill::run(command),
        Commands::Compliance { command } => complai::compliance::run(command),
        Commands::System { command } => complai::system::run_system(command),
        Commands::Project { command } => complai::project::run_project(command),
        Commands::Fact { command } => complai::project::run_fact(command),
        Commands::Matrix { command } => complai::project::run_matrix(command),
        Commands::Evidence { command } => complai::project::run_evidence(command),
        Commands::Gen { command } => complai::reports::run_gen(command),
        Commands::Parse { file } => complai::parse::parse(&file),
    }
}
