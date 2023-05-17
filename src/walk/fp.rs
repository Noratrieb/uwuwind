//! Test frame pointer walker. Not very good.

use crate::arch::get_rbp;

pub(crate) unsafe fn walk() {
    let mut current_rbp = get_rbp();
    loop {
        println!("walk...   rbp={current_rbp:p}");

        let return_addr = current_rbp.add(1).read() as *const usize;
        println!("walk... return_addr={return_addr:p}");

        println!(
            "walk... frame={:?}",
            crate::identify::identify(return_addr as usize)
        );

        println!("no read yet");

        current_rbp = current_rbp.read() as *const usize;
    }
}
