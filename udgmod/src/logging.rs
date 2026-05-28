use std::{fs::File, io::Write as _};

use eyre::Result;
use log::{Level, LevelFilter, Log, Metadata, Record};
use parking_lot::Mutex;

struct Logger {
    console_level: LevelFilter,
    file_level: LevelFilter,
    file: Mutex<File>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.console_level || metadata.level() <= self.file_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = format!("[{:5}]", record.level());
        let target = record.target();
        let args = format!("{}", record.args());

        if record.level() <= self.console_level {
            let line = format!("{level} {args}");
            match record.level() {
                Level::Error | Level::Warn => eprintln!("{line}"),
                _ => println!("{line}"),
            }
        }

        if record.level() <= self.file_level {
            let mut file = self.file.lock();
            writeln!(file, "{level} {target} - {args}").ok();
            file.flush().ok();
        }
    }

    fn flush(&self) {
        let mut file = self.file.lock();
        file.flush().ok();
    }
}

pub fn init(path: &str, console_level: LevelFilter, file_level: LevelFilter) -> Result<()> {
    let file = File::create(path)?;

    let logger = Logger {
        console_level,
        file_level,
        file: Mutex::new(file),
    };

    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(console_level.max(file_level));

    Ok(())
}
