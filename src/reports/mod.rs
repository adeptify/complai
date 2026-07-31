//! 报告与交付物生成。

pub mod report;

use crate::cli::GenCommand;

pub fn run_gen(cmd: GenCommand) -> eyre::Result<()> {
    match cmd {
        GenCommand::Report => report::generate(),
    }
}
