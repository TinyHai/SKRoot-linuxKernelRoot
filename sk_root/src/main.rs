#[cfg(target_os = "android")]
use android_logger::Config;

use log::LevelFilter;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Trace) // limit log level
            .with_tag("SKRoot"), // logs will show under mytag tag
    );
    sk_root::root::root_shell()
}
