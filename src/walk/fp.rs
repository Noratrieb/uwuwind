//! Test frame pointer walker. Not very good.

use crate::arch::get_rbp;
use crate::stdext::trace;

pub(crate) unsafe fn walk() {
    let mut current_rbp = get_rbp().0.cast::<usize>();
    loop {
        trace!("walk...   rbp={current_rbp:p}");

        let return_addr = current_rbp.add(1).read() as *const usize;
        trace!("walk... return_addr={return_addr:p}");

        trace!(
            "walk... frame={:?}",
            crate::identify::identify(return_addr as usize)
        );

        trace!("no read yet");

        current_rbp = current_rbp.read() as *const usize;
    }
}
