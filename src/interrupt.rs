use crossbeam_channel::{bounded, Receiver};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

static INTERRUPT_SIGNAL: AtomicBool = AtomicBool::new(false);
static INTERRUPT_NOTIFIER: Lazy<Arc<InterruptNotifier>> =
    Lazy::new(|| Arc::new(InterruptNotifier::new()));

struct InterruptNotifier {
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl InterruptNotifier {
    fn new() -> Self {
        Self {
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    fn create_channel(&self) -> Receiver<()> {
        let (sender, receiver) = bounded(1);
        let (ready_sender, ready_receiver) = bounded(0);
        let notifier = Arc::clone(&INTERRUPT_NOTIFIER);

        std::thread::spawn(move || {
            let mut _guard = notifier.mutex.lock().unwrap();
            // Send a ready signal AFTER acquiring the mutex
            let _ = ready_sender.send(());
            loop {
                // Waiting on the condvar will block the thread AND release the mutex
                // Then when the condvar is notified, it will re-acquire the mutex
                // and continue the loop
                _guard = notifier.condvar.wait(_guard).unwrap();
                if INTERRUPT_SIGNAL.load(Ordering::SeqCst) {
                    let _ = sender.send(());
                    return;
                }
            }
        });

        // Wait for the thread to be ready (acquire the mutex)
        let _ = ready_receiver.recv();
        receiver
    }
}

pub fn setup_interrupt_handler() {
    ctrlc::set_handler(move || {
        INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
        // Notify all waiting threads
        // This acquires the mutex, which means any channels must be waiting on the condvar
        // and will be notified
        let _guard = INTERRUPT_NOTIFIER.mutex.lock().unwrap();
        INTERRUPT_NOTIFIER.condvar.notify_all();
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
    INTERRUPT_NOTIFIER.create_channel()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_interrupt() {
        assert!(check_interrupt().is_ok());

        INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
        assert_eq!(check_interrupt(), Err("Operation cancelled by user"));

        // This must be reset for other tests
        INTERRUPT_SIGNAL.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_interrupt_channel() {
        let receiver = get_interrupt_channel();
        // Initially should not receive anything
        assert!(receiver.try_recv().is_err());

        // Manually trigger interrupt
        {
            INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
            let _guard = INTERRUPT_NOTIFIER.mutex.lock().unwrap();
            INTERRUPT_NOTIFIER.condvar.notify_all();
        }

        // Should receive notification
        assert!(receiver.recv().is_ok());

        // Reset for other tests
        INTERRUPT_SIGNAL.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_multiple_interrupt_channels() {
        let receiver1 = get_interrupt_channel();
        let receiver2 = get_interrupt_channel();
        let receiver3 = get_interrupt_channel();
        assert!(receiver1.try_recv().is_err());
        assert!(receiver2.try_recv().is_err());
        assert!(receiver3.try_recv().is_err());

        {
            INTERRUPT_SIGNAL.store(true, Ordering::SeqCst);
            let _guard = INTERRUPT_NOTIFIER.mutex.lock().unwrap();
            INTERRUPT_NOTIFIER.condvar.notify_all();
        }

        // All receivers should get the signal
        assert!(receiver1.recv().is_ok());
        assert!(receiver2.recv().is_ok());
        assert!(receiver3.recv().is_ok());

        // Reset for other tests
        INTERRUPT_SIGNAL.store(false, Ordering::SeqCst);
    }
}
