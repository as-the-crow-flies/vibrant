use pollster::FutureExt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .try_init()
        .unwrap();

    vibrant::app::run().block_on();
}
