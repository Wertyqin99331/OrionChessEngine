use std::{
    io::Write,
    sync::{Arc, Mutex, OnceLock},
};

static OUT: OnceLock<Arc<Mutex<dyn Write + Send>>> = OnceLock::new();

pub fn init_out(w: impl Write + Send + 'static) {
    OUT.set(Arc::new(Mutex::new(w))).ok();
}

pub fn write_line(s: &str) {
    if let Some(out) = OUT.get() {
        let mut w = out.lock().unwrap();

        writeln!(w, "{s}").ok();
        w.flush().ok();
    }
}
