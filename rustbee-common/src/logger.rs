use std::fs::{self, File};
use std::io::{Read, SeekFrom, Write};

use tokio::fs::File as AsyncFile;
use tokio::io::{AsyncBufReadExt as _, AsyncSeekExt as _, BufReader as AsyncBufReader};

use log::{Level, Log, Metadata, Record};

use crate::constants::{LOG_LEVEL, LOG_PATH};

pub use log::{debug, error, info, trace, warn};

const MAX_TAIL_LINES: usize = 50;

pub struct Logger {
    name: &'static str,
    use_stdout_stderr: bool,
}

impl Logger {
    pub const fn new(name: &'static str, use_stdout_stderr: bool) -> Self {
        Self {
            name,
            use_stdout_stderr,
        }
    }

    pub fn init(&'static self) {
        log::set_logger(self).expect("Unexpected error: Cannot set logger twice");
        log::set_max_level(log::LevelFilter::Trace);
    }

    /// If tail specified, prints the last x lines too before awaiting the next lines
    pub async fn follow(&self, tail: Option<usize>) {
        println!("Waiting for log content, press CTRL+C or send SIGINT to exit");

        if tail.is_some() {
            self.print(tail);
        }

        let mut file = AsyncFile::open(LOG_PATH).await.unwrap();
        let mut reader = AsyncBufReader::new(file.try_clone().await.unwrap());

        file.seek(SeekFrom::End(0)).await.unwrap();

        loop {
            let mut line = String::new();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    // Gracefully and implicitly drops file handles
                    return;
                }
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(0) => continue,
                        Ok(_) => print!("{line}"),
                        Err(err) => {
                            error!("Error while reading file: {err}");
                            return;
                        }
                    }
                }
            };
        }
    }

    pub fn print(&self, tail: Option<usize>) {
        let mut file =
            if !fs::exists(LOG_PATH).expect("Lack permissions to check if log file exists") {
                File::create_new(LOG_PATH).unwrap_or_else(|err| {
                    panic!("Unexpected error: Cannot create the log file at {LOG_PATH}: {err}")
                })
            } else {
                File::open(LOG_PATH).unwrap_or_else(|err| {
                    panic!(
                    "Unexpected error: Cannot get a (write) handle to log file at {LOG_PATH}: {err}"
                )
                })
            };

        let mut content = String::new();
        file.read_to_string(&mut content)
            .expect("Failed to read log file");

        if tail.is_some_and(|v| v <= MAX_TAIL_LINES) {
            content
                .lines()
                .rev()
                .enumerate()
                .take_while(|(i, _)| *i < tail.unwrap())
                .collect::<Vec<_>>()
                .iter()
                .rev()
                .for_each(|(_, line)| println!("{line}"));

            return;
        }

        print!("{content}");
    }

    pub fn purge(&self) {
        if !fs::exists(LOG_PATH).expect("Lack permissions to check if log file exists") {
            return;
        }

        File::options()
            .write(true)
            .truncate(true)
            .open(LOG_PATH)
            .unwrap_or_else(|err| {
                panic!(
                    "Unexpected error: Cannot get a (write) handle to log file at {LOG_PATH}: {err}"
                )
            });
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut file = File::options()
            .create(true)
            .append(true)
            .open(LOG_PATH)
            .unwrap_or_else(|err| {
                panic!(
                    "Unexpected error: Cannot get a (write) handle to log file at {LOG_PATH}: {err}"
                )
            });

        let content = format!("{}\n", record.args());
        let log_content = format!(
            "[{}]<{}> {}: {}",
            self.name,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            content
        );

        if self.use_stdout_stderr {
            match record.level() {
                Level::Error | Level::Warn => eprint!("{content}"),
                _ => print!("{content}"),
            }
        }

        file.write_all(log_content.as_bytes())
            .expect("Unexpected error: Failed to write to log file");
        file.flush().unwrap();
    }

    fn flush(&self) {}
}
