struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if cfg!(debug_assertions) {
            true
        } else {
            metadata.level() <= log::Level::Info
        }
    }

    fn log(&self, record: &log::Record) {
        use log::Level;
        const RESET: &str = "\x1b[0m";
        const INFO_COLOR: &str = "\x1b[36m";
        const WARN_COLOR: &str = "\x1b[33m";
        const ERROR_COLOR: &str = "\x1b[1;31m";

        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Trace | Level::Debug => {
                    println!("{}", record.args());
                }
                Level::Info => {
                    println!("{}{}{}", INFO_COLOR, record.args(), RESET);
                }
                Level::Warn => {
                    println!("{}{}{}", WARN_COLOR, record.args(), RESET);
                }
                Level::Error => {
                    println!("{}{}{}", ERROR_COLOR, record.args(), RESET);
                }
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub(crate) fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
}
