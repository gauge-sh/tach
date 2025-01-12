use crossbeam_channel::{bounded, Receiver, Sender};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

static INTERRUPT_SIGNAL: AtomicBool = AtomicBool::new(false);
static INTERRUPT_CHANNEL: Lazy<(Sender<()>, Receiver<()>)> = Lazy::new(|| bounded(1));

pub fn setup_interrupt_handler() {
    ctrlc::set_handler(move || {
        INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
        let _ = INTERRUPT_CHANNEL.0.send(());
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

pub fn get_interrupt_channel() -> Receiver<()> {
    INTERRUPT_CHANNEL.1.clone()
}
