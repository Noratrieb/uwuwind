use core::arch::asm;

use crate::Addr;

pub(crate) fn get_rbp() -> Addr {
    let mut out;
    unsafe {
        asm!(
            "mov {out}, rbp",
            out = out(reg) out,
            options(nostack, readonly)
        );
    }
    Addr(out)
}

pub(crate) fn get_rip() -> Addr {
    let mut out;
    unsafe {
        asm!(
            "lea {out}, [rip]",
            out = out(reg) out,
            options(nostack, readonly),
        );
    }
    Addr(out)
}

pub(crate) struct Context {
    pub(crate) registers: [usize; 32],
}

pub(crate) fn capture_context() -> Context {
    let mut context = Context { registers: [0; 32] };

    unsafe {
        asm!(
            "mov [{regs}+1*8], rdx", // required special
            "mov [{regs}+2*8], rcx", // required special
            "mov [{regs}+3*8], rbx", // required callee-saved
            "mov [{regs}+4*8], rsi", // required special
            "mov [{regs}+5*8], rdi", // required special
            "mov [{regs}+6*8], rbp", // required callee-saved
            "mov [{regs}+7*8], rsp", // required callee-saved

            "mov [{regs}+12*8], r12", // required callee-saved
            "mov [{regs}+13*8], r13", // required callee-saved
            "mov [{regs}+14*8], r14", // required callee-saved
            "mov [{regs}+15*8], r15", // required callee-saved

            "lea rax, [rip + 0]", // must use rip as a base register
            "mov [{regs}+16*8], rax", // return address

            out ("rax") _, // clobbers rax
            regs = in(reg) &mut context.registers,
            options(readonly),
        );
    }

    context
}
