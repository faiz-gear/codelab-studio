use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use super::{Process, Spawn, output::Capture};

pub(super) type ErrorLog = Arc<Mutex<VecDeque<String>>>;

pub(super) struct TerminalProcess {
    process: Process,
    input: mpsc::SyncSender<String>,
    reader: Option<JoinHandle<()>>,
}

impl TerminalProcess {
    pub fn open(spec: Spawn, capture: Arc<Mutex<Capture>>, errors: ErrorLog) -> io::Result<Self> {
        let (process, (mut read, mut write)) = Process::spawn(spec)?;
        let reader = thread::spawn(move || {
            let mut buffer = [0; 32 * 1024];
            loop {
                match read.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => lock(&capture).append(&buffer[..count], &mut lock(&errors)),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            lock(&capture).finish_line(&mut lock(&errors));
        });
        let (input, rx) = mpsc::sync_channel::<String>(32);
        thread::spawn(move || {
            for text in rx {
                if write.write_all(text.as_bytes()).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            process,
            input,
            reader: Some(reader),
        })
    }

    pub fn send(&self, text: String) -> io::Result<()> {
        if text.len() > 128 * 1024 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.input
            .try_send(text)
            .map_err(|_| io::ErrorKind::WouldBlock.into())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        self.process
            .resize(cols.clamp(2, 1_000), rows.clamp(1, 1_000))
    }

    pub fn exit_code(&mut self) -> Option<u32> {
        self.process.reap_if_exited().inspect(|_| self.drain())
    }

    pub fn stop(&mut self) -> u32 {
        #[cfg(windows)]
        let _ = self.send("\u{3}".into());
        let code = self.process.interrupt();
        self.drain();
        code
    }

    fn drain(&mut self) {
        // The kernel closes the slave endpoint when the last member exits.
        // Poll the join handle so unexpected orphaned descriptors cannot hang IPC.
        if let Some(reader) = self.reader.take() {
            for _ in 0..40 {
                if reader.is_finished() {
                    let _ = reader.join();
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}
