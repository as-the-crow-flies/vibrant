use clap::Parser;
use pollster::FutureExt;

pub mod app;
pub mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod video;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .try_init()
        .unwrap();

    let args = cli::CliArgs::parse();
    app::run(args.into_config()).block_on();
}
