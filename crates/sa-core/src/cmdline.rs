//! The unnamed command-line splitter at 0x746480 (WinMain: `argv = f(&argc)` with the string from
//! `GetCommandLineA` in EAX), named `Win::CommandLineToArgv` here. gta-reversed has no counterpart
//! (its WinMain uses the CRT's `__argv`); ported from the 1.0 US bytes.

use crate::abi::{HookReg, entry_eax};
use crate::hook;
use crate::mem::{GPtr, GlobalDesc};

/// Calls into other functions.
pub mod call {
    pub use crate::memory::malloc;
}

/// A separator in the original's sense: `c <= ' '` (unsigned, so bytes 0x80..=0xFF are word
/// characters) outside quotes, or the terminating position `i == len`.
fn is_separator(c: u8, in_quote: bool, i: i32, len: i32) -> bool {
    (c <= b' ' && !in_quote) || i == len
}

/// `Win::CommandLineToArgv(const char* cmdLine /* EAX */, int* argc)` (0x746480). The pure body; the
/// hooked entry is [`command_line_to_argv_entry`].
///
/// Per the bytes: `len = strlen(cmdLine)`. Pass 1 counts words over `i = 0..=len` (the terminator
/// included): `"` toggles the in-quote flag (and is a word character itself); a separator (see
/// [`is_separator`]) ends a word in progress. Then one `malloc(len + count * 4 + 1)` block holds the
/// `count` pointers followed by a `strcpy` of the command line. Pass 2 walks the ORIGINAL string again
/// with its own quote flag: a non-separator that is not `"` starts a word when none is in progress
/// (`*argv++ = &copy[i]`; a leading `"` is skipped this way); a separator ending a word writes a NUL
/// over `copy[i - 1]` if that is `"` (drops one closing quote), else over `copy[i]`. Finally
/// `*argc = count` (pass 1's count) and the block is returned.
///
/// Original quirks kept: a word made only of quotes (`""`) is counted by pass 1 but never stored by
/// pass 2, so the last `argv` slots are left as malloc garbage while `argc` counts them; no NULL
/// terminator after the pointers; inner quotes stay in the strings (`a"b c"d` → `a"b c"d`); a
/// negative `strlen` (≥ 2 GiB string) skips both passes. A null `malloc` result is not checked.
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
pub fn command_line_to_argv(cmd: GPtr<u8>, argc: GPtr<i32>) -> GPtr<GPtr<u8>> {
    let ch = |i: i32| cmd.byte_add(i as isize).read();
    let mut len: i32 = 0;
    while ch(len) != 0 {
        len = len.wrapping_add(1);
    }

    // pass 1: count
    let mut count: i32 = 0;
    let (mut in_quote, mut in_word) = (false, false);
    let mut i: i32 = 0;
    if len >= 0 {
        while i <= len {
            let c = ch(i);
            if c == b'"' {
                in_quote = !in_quote;
            }
            if is_separator(c, in_quote, i, len) {
                if in_word {
                    in_word = false;
                    count += 1;
                }
            } else if !in_word {
                in_word = true;
            }
            i += 1;
        }
    }

    let slot = core::mem::size_of::<GPtr<u8>>() as u32; // 4 on the original's targets
    let size = (len as u32)
        .wrapping_add((count as u32).wrapping_mul(slot))
        .wrapping_add(1);
    let argv: GPtr<GPtr<u8>> = call::malloc(size).cast();
    let copy: GPtr<u8> = argv.byte_add((count as u32).wrapping_mul(slot) as isize).cast();
    // strcpy(copy, cmd)
    let mut k: isize = 0;
    loop {
        let c = cmd.byte_add(k).read();
        copy.byte_add(k).write(c);
        if c == 0 {
            break;
        }
        k += 1;
    }

    if len < 0 {
        argc.write(count);
        return argv;
    }
    // pass 2: split
    let mut out = argv;
    let (mut in_quote, mut in_word) = (false, false);
    let mut i: i32 = 0;
    while i <= len {
        let c = ch(i);
        if c == b'"' {
            in_quote = !in_quote;
        }
        if is_separator(c, in_quote, i, len) {
            if in_word {
                let prev = copy.byte_add(i as isize - 1);
                if prev.read() == b'"' {
                    prev.write(0);
                } else {
                    copy.byte_add(i as isize).write(0);
                }
                in_word = false;
            }
        } else if !in_word && c != b'"' {
            out.write(copy.byte_add(i as isize));
            in_word = true;
            out = out.byte_add(slot as isize);
        }
        i += 1;
    }
    argc.write(count);
    argv
}

/// The hooked entry of 0x746480: cdecl with one stack argument (`int* argc`) and the command line in
/// **EAX** (whole-program-optimised call from WinMain at 0x748773), read through `entry_eax`. Returns
/// `argv` in EAX. See [`command_line_to_argv`].
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x746480, conv = cdecl, name = "FUN_00746480", expect = "83 EC 14 53 55 56", entry_eax)]
pub fn command_line_to_argv_entry(argc: GPtr<i32>) -> GPtr<GPtr<u8>> {
    command_line_to_argv(GPtr::from_addr(entry_eax() as usize), argc)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COMMAND_LINE_TO_ARGV_ENTRY];

/// Globals owned by this module (none).
pub const GLOBALS: &[GlobalDesc] = &[];

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::orig::host;
    use std::string::String;
    use std::vec::Vec;

    fn leak_malloc(size: u32) -> GPtr<u8> {
        // garbage-filled like a fresh heap block
        let v: Vec<u8> = std::vec![0xCD; size as usize];
        GPtr::from_raw(std::boxed::Box::leak(v.into_boxed_slice()).as_mut_ptr())
    }

    fn split(s: &str) -> (i32, Vec<String>) {
        let mut buf: Vec<u8> = s.as_bytes().to_vec();
        buf.push(0);
        let mut argc = -1;
        let argv = command_line_to_argv(GPtr::from_raw(buf.as_mut_ptr()), GPtr::from_raw(&raw mut argc));
        let mut out = Vec::new();
        for n in 0..argc {
            let p = argv.wrapping_index(n).read();
            if p.addr() == usize::MAX / 0xFF * 0xCD {
                // a malloc garbage slot (0xCDCDCDCD; pointer-sized 0xCD bytes on a 64-bit host)
                out.push(String::from("<garbage>"));
                continue;
            }
            let mut s = String::new();
            let mut k = 0;
            loop {
                let c = p.byte_add(k).read();
                if c == 0 {
                    break;
                }
                s.push(c as char);
                k += 1;
            }
            out.push(s);
        }
        (argc, out)
    }

    #[test]
    fn splits_like_the_original() {
        host::set::<fn(u32) -> GPtr<u8>>("sa_core::memory::malloc", leak_malloc);
        assert_eq!(split(""), (0, Vec::new()));
        assert_eq!(split("gta_sa.exe"), (1, std::vec![String::from("gta_sa.exe")]));
        let (n, v) = split("\"C:\\Games\\GTA San Andreas\\gta_sa.exe\"  -vms\t-x");
        assert_eq!(n, 3);
        assert_eq!(v, ["C:\\Games\\GTA San Andreas\\gta_sa.exe", "-vms", "-x"]);
        // inner quotes stay; only a closing quote right before the separator is dropped
        assert_eq!(split("a\"b c\"d").1, ["a\"b c\"d"]);
        // a quotes-only word is counted but never stored
        let (n, v) = split("a \"\" b");
        assert_eq!(n, 3);
        assert_eq!(v, ["a", "b", "<garbage>"]);
        host::clear("sa_core::memory::malloc");
    }
}
