use std::ptr;
use error;

extern "C" {
    static InterruptPending: u8; // C bool
    fn ProcessInterrupts();
}

// TODO: proper win32 support (does slightly more)


#[inline(always)]
pub fn check_for_interrupts() {
    unsafe {
        let pending = ptr::read_volatile(&InterruptPending) != 0;
        if pending {
            error::convert_postgres_error(|| ProcessInterrupts());
        }
    }
}
