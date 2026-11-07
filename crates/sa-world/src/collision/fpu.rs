//! A literal x87 register-stack machine over [`sa_math::x87::X87`] (PC = 24), so that the COLL ports
//! can follow the original instruction sequences one instruction at a time (operand forms, register
//! reuse, which values are stored to memory as f32 and which stay on the stack).
//!
//! Naming follows the Intel forms the disassembly shows:
//! - `fadd_m(m)` .. `fdivr_m(m)`: `fadd dword [m]` etc. (`st0 = st0 op m`, `fsubr`/`fdivr`: `m op st0`);
//! - `fadd_st(i)` .. `fdivr_st(i)`: `fadd st(0), st(i)` (D8 forms: `st0 = st0 op st(i)`, `fsubr`:
//!   `st(i) - st0`);
//! - `faddp(i)` .. `fdivrp(i)`: `faddp st(i), st(0)` (DE forms: `st(i) = st(i) op st0`, then pop;
//!   `fsubrp`/`fdivrp`: `st0 op st(i)`);
//! - compares set the condition codes that `fnstsw ax` puts into AH ([`Fpu::ah`]).

use sa_math::x87::{Cmp, X87};

/// The x87 register stack (8 registers) and the condition codes of the last compare.
#[derive(Clone, Copy, Debug)]
pub struct Fpu {
    st: [X87; 8],
    n: usize,
    cmp: Cmp,
}

impl Default for Fpu {
    fn default() -> Self {
        Fpu::new()
    }
}

impl Fpu {
    pub fn new() -> Fpu {
        Fpu {
            st: [X87::zero(); 8],
            n: 0,
            cmp: Cmp::Equal,
        }
    }

    /// Number of values on the stack.
    pub fn depth(&self) -> usize {
        self.n
    }

    /// `st(i)`.
    pub fn st(&self, i: usize) -> X87 {
        assert!(i < self.n, "x87 stack underflow");
        self.st[self.n - 1 - i]
    }

    fn set(&mut self, i: usize, v: X87) {
        assert!(i < self.n, "x87 stack underflow");
        self.st[self.n - 1 - i] = v;
    }

    /// Pushes a register value.
    pub fn push(&mut self, v: X87) {
        assert!(self.n < 8, "x87 stack overflow");
        self.st[self.n] = v;
        self.n += 1;
    }

    /// Pops st0.
    pub fn pop(&mut self) -> X87 {
        assert!(self.n > 0, "x87 stack underflow");
        self.n -= 1;
        self.st[self.n]
    }

    /// `fld dword`
    pub fn fld(&mut self, m: f32) {
        self.push(X87::from_f32(m));
    }
    /// `fld st(i)`
    pub fn fld_st(&mut self, i: usize) {
        let v = self.st(i);
        self.push(v);
    }
    /// `fild dword`
    pub fn fild(&mut self, v: i32) {
        self.push(X87::from_i32(v));
    }
    /// `fld1`
    pub fn fld1(&mut self) {
        self.push(X87::one());
    }
    /// `fldz`
    pub fn fldz(&mut self) {
        self.push(X87::zero());
    }

    pub fn fadd_m(&mut self, m: f32) {
        let v = self.st(0).fadd_m(m);
        self.set(0, v);
    }
    pub fn fsub_m(&mut self, m: f32) {
        let v = self.st(0).fsub_m(m);
        self.set(0, v);
    }
    pub fn fsubr_m(&mut self, m: f32) {
        let v = self.st(0).fsubr_m(m);
        self.set(0, v);
    }
    pub fn fmul_m(&mut self, m: f32) {
        let v = self.st(0).fmul_m(m);
        self.set(0, v);
    }
    pub fn fdiv_m(&mut self, m: f32) {
        let v = self.st(0).fdiv_m(m);
        self.set(0, v);
    }
    pub fn fdivr_m(&mut self, m: f32) {
        let v = self.st(0).fdivr_m(m);
        self.set(0, v);
    }

    pub fn fadd_st(&mut self, i: usize) {
        let v = self.st(0).fadd(self.st(i));
        self.set(0, v);
    }
    pub fn fsub_st(&mut self, i: usize) {
        let v = self.st(0).fsub(self.st(i));
        self.set(0, v);
    }
    pub fn fsubr_st(&mut self, i: usize) {
        let v = self.st(0).fsubr(self.st(i));
        self.set(0, v);
    }
    pub fn fmul_st(&mut self, i: usize) {
        let v = self.st(0).fmul(self.st(i));
        self.set(0, v);
    }
    pub fn fdiv_st(&mut self, i: usize) {
        let v = self.st(0).fdiv(self.st(i));
        self.set(0, v);
    }
    pub fn fdivr_st(&mut self, i: usize) {
        let v = self.st(0).fdivr(self.st(i));
        self.set(0, v);
    }

    /// DC forms `fadd st(i), st(0)`: `st(i) = st(i) + st0` (no pop).
    pub fn fadd_to(&mut self, i: usize) {
        let v = self.st(i).fadd(self.st(0));
        self.set(i, v);
    }
    /// `fmul st(i), st(0)`
    pub fn fmul_to(&mut self, i: usize) {
        let v = self.st(i).fmul(self.st(0));
        self.set(i, v);
    }

    pub fn faddp(&mut self, i: usize) {
        let v = self.st(i).fadd(self.st(0));
        self.set(i, v);
        self.pop();
    }
    pub fn fsubp(&mut self, i: usize) {
        let v = self.st(i).fsub(self.st(0));
        self.set(i, v);
        self.pop();
    }
    pub fn fsubrp(&mut self, i: usize) {
        let v = self.st(i).fsubr(self.st(0));
        self.set(i, v);
        self.pop();
    }
    pub fn fmulp(&mut self, i: usize) {
        let v = self.st(i).fmul(self.st(0));
        self.set(i, v);
        self.pop();
    }
    pub fn fdivp(&mut self, i: usize) {
        let v = self.st(i).fdiv(self.st(0));
        self.set(i, v);
        self.pop();
    }
    pub fn fdivrp(&mut self, i: usize) {
        let v = self.st(i).fdivr(self.st(0));
        self.set(i, v);
        self.pop();
    }

    pub fn fchs(&mut self) {
        let v = self.st(0).fchs();
        self.set(0, v);
    }
    /// `fabs` (clears the sign, NaNs included).
    pub fn fabs(&mut self) {
        let b = self.st(0).to_f64().to_bits() & !(1u64 << 63);
        self.set(0, X87::from_f64_exact(f64::from_bits(b)));
    }
    pub fn fsqrt(&mut self) {
        let v = self.st(0).fsqrt();
        self.set(0, v);
    }
    /// `fxch st(i)`
    pub fn fxch(&mut self, i: usize) {
        let (a, b) = (self.st(0), self.st(i));
        self.set(0, b);
        self.set(i, a);
    }
    /// `fstp st(i)`: `st(i) = st0`, pop (`fstp st(0)` just pops).
    pub fn fstp_st(&mut self, i: usize) {
        let v = self.st(0);
        self.set(i, v);
        self.pop();
    }
    /// `fst dword`
    pub fn fst(&self) -> f32 {
        self.st(0).to_f32()
    }
    /// `fstp dword`
    pub fn fstp(&mut self) -> f32 {
        self.pop().to_f32()
    }

    /// `fcom dword [m]`
    pub fn fcom_m(&mut self, m: f32) {
        self.cmp = self.st(0).fcom(X87::from_f32(m));
    }
    /// `fcomp dword [m]`
    pub fn fcomp_m(&mut self, m: f32) {
        self.fcom_m(m);
        self.pop();
    }
    /// `fcom st(i)`
    pub fn fcom_st(&mut self, i: usize) {
        self.cmp = self.st(0).fcom(self.st(i));
    }
    /// `fcomp st(i)`
    pub fn fcomp_st(&mut self, i: usize) {
        self.fcom_st(i);
        self.pop();
    }
    /// `fcompp` (st0 against st1, pop both)
    pub fn fcompp(&mut self) {
        self.fcom_st(1);
        self.pop();
        self.pop();
    }

    /// AH after `fnstsw ax` (C0 0x01, C2 0x04, C3 0x40).
    pub fn ah(&self) -> u8 {
        self.cmp.ah()
    }
    /// `test ah, mask; jp`: taken when the masked bits have even parity.
    pub fn jp(&self, mask: u8) -> bool {
        self.cmp.test_parity(mask)
    }
    /// `test ah, mask; jne`: taken when any masked bit is set.
    pub fn jne(&self, mask: u8) -> bool {
        self.cmp.test_any(mask)
    }
}
