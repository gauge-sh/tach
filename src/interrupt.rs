use std::sync::atomic::{AtomicBool, Ordering};

pub static INTERRUPT_SIGNAL: AtomicBool = AtomicBool::new(false);

pub fn setup_interrupt_handler() {
    ctrlc::set_handler(move || {
        INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

pub fn check_interrupt() -> Result<(), &'static str> {
    if INTERRUPT_SIGNAL.load(Ordering::SeqCst) {
        Err("Operation cancelled by user")
    } else {
        Ok(())
    }
}
