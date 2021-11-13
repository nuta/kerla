use atomic_refcell::AtomicRefCell;
use log_filter::LogFilter;

struct Logger {
    filter: AtomicRefCell<LogFilter>,
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        use log::Level;
        const RESET: &str = "\x1b[0m";
        const INFO_COLOR: &str = "\x1b[36m";
        const WARN_COLOR: &str = "\x1b[33m";
        const ERROR_COLOR: &str = "\x1b[1;31m";

        if !self.filter.borrow().should_print(record) {
            return;
        }

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

    fn flush(&self) {}
}

static LOGGER: Logger = Logger {
    filter: AtomicRefCell::new(LogFilter::empty()),
};

pub fn set_log_filter(pattern: &str) {
    let new_filter = LogFilter::new(pattern);
    *LOGGER.filter.borrow_mut() = new_filter;
}

pub(crate) fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
}
