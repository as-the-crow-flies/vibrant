use pollster::FutureExt;

pub mod app;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .try_init()
        .unwrap();

    app::run().block_on();
}
