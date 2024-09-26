use futures::executor::block_on;

pub mod app;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .unwrap();

    block_on(app::run())
}
