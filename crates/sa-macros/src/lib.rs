//! `#[hook]`: registers a ported function as the Rust side of one original function (ADR-001).
//!
//! ```ignore
//! #[hook(addr = 0x407180, conv = cdecl, name = "CGeneral::GetRandomNumberInRange_int32",
//!        expect = "51 56 E8 97 A9 41 00")]   // thiscall: also `this = "CPool<int32>"`
//! pub fn get_random_number_in_range_i32(min: i32, max: i32) -> i32 { ... }
//! ```
//!
//! The function itself is left unchanged (plain Rust, callable and testable on every target). Next to
//! it the macro emits:
//! - on `target_arch = "x86"` only: a shim `extern "<conv>" fn` with the original calling convention
//!   (`cdecl` = `"C"`, `stdcall`, `thiscall` with `this` as the first argument, `fastcall`). It counts
//!   the call, runs the function under the engine's FPU environment (`sa_core::abi::FpuGuard`) and
//!   converts the return value with `sa_core::abi::IntoAbi` (e.g. an x87 value goes back in ST0 with
//!   all its bits). Other targets get no shim, so the thiscall/fastcall ABIs never reach them.
//! - `pub const HOOK_<NAME>: sa_core::abi::HookReg`: address, catalogue name, convention, the entry
//!   bytes the installer must find, the shim address (0 off x86) and the call counter. The crate lists
//!   these constants explicitly (`sa_core::hooks::ALL`); `sa-hook` installs them.
//!
//! `preserve = "edx"` (or `"ecx"`, `"ecx,edx"`): some original callers rely on the original function
//! leaving a scratch register unchanged (whole-program-optimised call sites, e.g. 0x5B4177 tests DL
//! after calling 0x5B3AD0). The entry is then a naked wrapper around the shim: it pushes the listed
//! registers, copies the stack arguments, calls the shim, pops the registers and returns with the
//! original convention (`ret` / `ret 4*n`). The shim's result registers (EAX, ST0) pass through.
//!
//! `footprint = W` (ADR-001 §5b): original code reads this function's stack residue. The entry is a
//! naked wrapper that snapshots the W bytes (≤ 0x1000, covering the shim's whole stack depth) below the entry ESP, calls the shim, restores
//! the snapshot and applies the writes the port recorded with `sa_core::abi::stack_footprint`.
//!
//! `peek = N` (PORT-HUD-3, ADR-001 §5b): the original reads stack slots it never wrote (an uninitialised
//! local: whatever the caller's earlier callees left there) or uses the address of one of its locals as
//! data. The entry is a naked wrapper that copies the N bytes (≤ `sa_core::abi::PEEK_MAX`) below the
//! entry ESP and the entry ESP itself into a per-depth frame before calling the shim (nesting safe up to
//! `sa_core::abi::PEEK_DEPTH`); the port reads them with `sa_core::abi::peek_entry_esp` /
//! `peek_stack`, which answer only while this hook's own frame is on top (`None` for a direct Rust call).
//!
//! `conv = reg("args = ebx esi; ret = al:entry; keep = ecx edx; pop")` (HARNESS-5, ADR-001 §4b,
//! `sa_core::regconv`): a compiler-generated register convention. The entry is a naked wrapper that
//! moves the registers and stack words into an `extern "C"` shim `(out, register words..., stack
//! arguments...)` and returns EAX/EDX/ST0 (80 bits for an `X87` result) from the shim's `RegOut`.
//! Cannot be combined with `preserve` / `entry_eax` / `footprint` / `peek` (use `keep`, `ret = ..:entry`).
//!
//! `fallback` (or `fallback = true`; ADR-002): a partial port. The macro also emits a cell that sa-hook
//! fills at install with the original entry (the relocated-prologue trampoline, or a `.HOODLUM` stub's
//! old target; `HookReg::set_original`) and `pub fn <fn>_original(args) -> R`, which calls that entry
//! with the original convention under the game's MXCSR (`abi::call_original`). The port calls it for
//! the paths it has not ported. Phases B/C have no original body: `<fn>_original` panics there (or
//! runs a host stand-in registered for `<module>::<fn>_original`). `R` must be its own ABI type
//! (integers, pointers, `()`), checked at compile time.
//!
//! `unverified = "BATCH"` (docs/PLAN.md "Writer/verifier split", docs/verify/HOWTO.md): the port is
//! written but not verified yet. It is compiled and registered like any other (`HookReg::batch` =
//! `Some(BATCH)`, so the table, callers, patch_refs and keycells tests still check it), but phase A
//! installs it only when selected (`SA_UNVERIFIED=all|BATCH|@FILE`, or named in `SA_HOOK_MODE`,
//! `SA_L2` or an `SA_HOOK_PORTS` list). BATCH is `[a-z0-9][a-z0-9.-]*` (one writer commit, or at most
//! ~60 hooks of one subsystem). An unverified hook must also carry the verifier's hints:
//! - `route = "L1-pure" | "L2" | "split" | "L3-only"`: how it is to be verified;
//! - `hazards = "none"` or a comma list of `x87-transcendental`, `stack-residue` (callers read the
//!   stack below their ESP after the call), `entry-protection-read` (the no-CD protection reads the
//!   entry bytes), `preserved-regs` (callers rely on ECX/EDX), `ax-al-return` (a 16/8-bit result with
//!   the caller's upper EAX), `uninit-locals` (the original reads uninitialised stack), `heap-writes`
//!   (writes heap memory that needs `tools/e2e/l2-extras.txt` regions);
//! - optional `recs = "grove-walk,driving"` (corpus recordings expected to call it) and
//!   `note = "..."` (anything else the verifier must know).
//!
//! `tools/verify/queue.py` reads these attributes into `catalog/verify-queue.csv`.
//!
//! Generated paths start with `::sa_core`, so the crate that uses `#[hook]` must have `sa_core` in
//! scope (sa-core itself declares `extern crate self as sa_core`).

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, LitInt, LitStr, Pat, ReturnType, parse_macro_input};

#[derive(Clone, Copy)]
enum Conv {
    Cdecl,
    Stdcall,
    Thiscall,
    Fastcall,
}

impl Conv {
    fn parse(i: &Ident) -> syn::Result<Conv> {
        Ok(match i.to_string().as_str() {
            "cdecl" => Conv::Cdecl,
            "stdcall" => Conv::Stdcall,
            "thiscall" => Conv::Thiscall,
            "fastcall" => Conv::Fastcall,
            _ => {
                return Err(syn::Error::new(
                    i.span(),
                    "conv must be cdecl, stdcall, thiscall or fastcall",
                ));
            }
        })
    }
    fn abi(self) -> LitStr {
        let s = match self {
            Conv::Cdecl => "C",
            Conv::Stdcall => "stdcall",
            Conv::Thiscall => "thiscall",
            Conv::Fastcall => "fastcall",
        };
        LitStr::new(s, Span::call_site())
    }
    fn variant(self) -> Ident {
        let s = match self {
            Conv::Cdecl => "Cdecl",
            Conv::Stdcall => "Stdcall",
            Conv::Thiscall => "Thiscall",
            Conv::Fastcall => "Fastcall",
        };
        Ident::new(s, Span::call_site())
    }
}

fn parse_expect(l: &LitStr) -> syn::Result<Vec<u8>> {
    let mut out = Vec::new();
    for t in l.value().split_whitespace() {
        let b = u8::from_str_radix(t, 16)
            .map_err(|_| syn::Error::new(l.span(), format!("`{t}` is not a hex byte")))?;
        out.push(b);
    }
    if out.len() < 5 {
        return Err(syn::Error::new(
            l.span(),
            "expect needs at least the 5 bytes the entry patch replaces",
        ));
    }
    Ok(out)
}

#[proc_macro_attribute]
pub fn hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut addr: Option<u32> = None;
    let mut conv: Option<Conv> = None;
    let mut name: Option<LitStr> = None;
    let mut expect: Option<Vec<u8>> = None;
    let mut this: Option<LitStr> = None;
    let mut preserve: Option<LitStr> = None;
    let mut entry_eax = false;
    let mut fallback = false;
    let mut footprint: Option<u32> = None;
    let mut peek: Option<u32> = None;
    let mut reg: Option<(LitStr, RegSpec)> = None;
    let mut hints = Hints::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("addr") {
            let v: LitInt = meta.value()?.parse()?;
            addr = Some(v.base10_parse::<u32>()?);
        } else if meta.path.is_ident("conv") {
            let input = meta.value()?;
            let v: Ident = input.parse()?;
            if v == "reg" {
                let content;
                syn::parenthesized!(content in input);
                let lit: LitStr = content.parse()?;
                let spec = RegSpec::parse(&lit)?;
                conv = Some(if spec.pop { Conv::Stdcall } else { Conv::Cdecl });
                reg = Some((lit, spec));
            } else {
                conv = Some(Conv::parse(&v)?);
            }
        } else if meta.path.is_ident("name") {
            name = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("expect") {
            let v: LitStr = meta.value()?.parse()?;
            expect = Some(parse_expect(&v)?);
        } else if meta.path.is_ident("this") {
            this = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("preserve") {
            preserve = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("footprint") {
            let v: LitInt = meta.value()?.parse()?;
            let w = v.base10_parse::<u32>()?;
            if w % 4 != 0 || w == 0 || w > 0x1000 {
                return Err(syn::Error::new(
                    v.span(),
                    "footprint: a multiple of 4, at most 0x1000",
                ));
            }
            footprint = Some(w);
        } else if meta.path.is_ident("unverified") {
            hints.batch = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("route") {
            hints.route = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("hazards") {
            hints.hazards = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("recs") {
            hints.recs = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("note") {
            hints.note = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("peek") {
            let v: LitInt = meta.value()?.parse()?;
            let w = v.base10_parse::<u32>()?;
            if w % 4 != 0 || w == 0 || w > 0x200 {
                return Err(syn::Error::new(v.span(), "peek: a multiple of 4, at most 0x200"));
            }
            peek = Some(w);
        } else if meta.path.is_ident("entry_eax") {
            entry_eax = true;
        } else if meta.path.is_ident("fallback") {
            fallback = if meta.input.peek(syn::Token![=]) {
                let v: syn::LitBool = meta.value()?.parse()?;
                v.value
            } else {
                true
            };
        } else {
            return Err(
                meta.error("expected addr, conv, name, expect, this, preserve, entry_eax, fallback, footprint, unverified, route, hazards, recs, note or peek")
            );
        }
        Ok(())
    });
    parse_macro_input!(attr with parser);
    let f = parse_macro_input!(item as ItemFn);
    if let Err(e) = hints.check() {
        return e.to_compile_error().into();
    }
    match expand(f, addr, conv, name, expect, this, preserve, entry_eax, fallback, footprint, peek, reg, &hints) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn expand(
    f: ItemFn,
    addr: Option<u32>,
    conv: Option<Conv>,
    name: Option<LitStr>,
    expect: Option<Vec<u8>>,
    this: Option<LitStr>,
    preserve: Option<LitStr>,
    entry_eax: bool,
    fallback: bool,
    footprint: Option<u32>,
    peek: Option<u32>,
    regconv: Option<(LitStr, RegSpec)>,
    hints: &Hints,
) -> syn::Result<proc_macro2::TokenStream> {
    let span = f.sig.ident.span();
    let missing = |what: &str| syn::Error::new(span, format!("#[hook] needs `{what} = ...`"));
    let addr = addr.ok_or_else(|| missing("addr"))?;
    let conv = conv.ok_or_else(|| missing("conv"))?;
    let name = name.ok_or_else(|| missing("name"))?;
    let expect = expect.ok_or_else(|| missing("expect"))?;
    let sig = &f.sig;
    if !sig.generics.params.is_empty() || sig.asyncness.is_some() || sig.variadic.is_some() {
        return Err(syn::Error::new(
            span,
            "#[hook] functions cannot be generic, async or variadic",
        ));
    }
    let mut args = Vec::new();
    let mut tys = Vec::new();
    for a in &sig.inputs {
        let FnArg::Typed(t) = a else {
            return Err(syn::Error::new(
                span,
                "#[hook] goes on free functions (pass `this` explicitly)",
            ));
        };
        let Pat::Ident(p) = &*t.pat else {
            return Err(syn::Error::new(
                span,
                "#[hook] arguments must be plain identifiers",
            ));
        };
        args.push(p.ident.clone());
        tys.push((*t.ty).clone());
    }
    let min_args = match conv {
        Conv::Thiscall => 1,
        _ => 0,
    };
    if args.len() < min_args {
        return Err(syn::Error::new(
            span,
            "a thiscall function takes `this` as its first argument",
        ));
    }
    // Arguments passed in registers: `this` (ECX) for thiscall, the first two for fastcall.
    let in_regs = match conv {
        Conv::Thiscall => 1,
        Conv::Fastcall => 2,
        _ => 0,
    };
    let stack_tys: Vec<_> = match &regconv {
        Some((lit, spec)) => {
            if !spec.args.is_empty() && spec.args.len() != args.len() {
                return Err(syn::Error::new(
                    lit.span(),
                    format!("reg conv: `args` lists {} locations for {} arguments", spec.args.len(), args.len()),
                ));
            }
            if preserve.is_some() || entry_eax || footprint.is_some() || peek.is_some() {
                return Err(syn::Error::new(
                    lit.span(),
                    "conv = reg(...) cannot be combined with preserve (use keep = ...), entry_eax (use ret = al:entry / ax:entry), footprint or peek",
                ));
            }
            tys.iter().enumerate().filter(|(i, _)| spec.loc(*i) == 0).map(|(_, t)| t).collect()
        }
        None => tys.iter().skip(in_regs).collect(),
    };
    let (this_type, this_size) = match &this {
        Some(t) => {
            let id: String = t
                .value()
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let id = Ident::new(id.trim_matches('_'), t.span());
            (quote!(Some(#t)), quote!(::sa_core::layouts::size::#id as u32))
        }
        None => (quote!(None), quote!(0)),
    };
    let ret = match &sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, t) => quote!(#t),
    };
    let fname = &sig.ident;
    let upper = fname.to_string().to_uppercase();
    let shim = format_ident!("__sa_hook_shim_{}", fname);
    let calls = format_ident!("__SA_HOOK_CALLS_{}", upper);
    let reg = format_ident!("HOOK_{}", upper);
    let abi = conv.abi();
    let abi_unwind = LitStr::new(&format!("{}-unwind", conv.abi().value()), Span::call_site());
    let native = format_ident!("__sa_hook_native_{}", fname);
    let variant = conv.variant();
    let mut doc = format!("Hook registration of `{}` ({addr:#x}).", name.value());
    if let Some(b) = &hints.batch {
        doc += &format!(
            " Unverified (batch `{}`, route {}, hazards {}): phase A installs it only when selected \
             (`SA_UNVERIFIED`).",
            b.value(),
            hints.route.as_ref().map(LitStr::value).unwrap_or_default(),
            hints.hazards.as_ref().map(LitStr::value).unwrap_or_default()
        );
    }
    let batch = match &hints.batch {
        Some(b) => quote!(Some(#b)),
        None => quote!(None),
    };
    // Registers to return unchanged, in push order.
    let mut saved: Vec<&'static str> = Vec::new();
    if let Some(p) = &preserve {
        for r in p.value().split(',').map(str::trim) {
            let r = match r {
                "ecx" => "ecx",
                "edx" => "edx",
                _ => {
                    return Err(syn::Error::new(
                        p.span(),
                        format!("preserve: `{r}` is not ecx or edx"),
                    ));
                }
            };
            if saved.contains(&r) {
                return Err(syn::Error::new(p.span(), format!("preserve: `{r}` twice")));
            }
            saved.push(r);
        }
        if saved.is_empty() {
            return Err(syn::Error::new(p.span(), "preserve: no register"));
        }
        let wide = matches!(&sig.output, ReturnType::Type(_, t) if matches!(quote!(#t).to_string().as_str(), "u64" | "i64"));
        if wide && saved.contains(&"edx") {
            return Err(syn::Error::new(
                p.span(),
                "preserve: EDX carries the upper half of a 64-bit result",
            ));
        }
    }
    let mask: u8 = saved
        .iter()
        .map(|r| if *r == "ecx" { 1u8 } else { 2u8 })
        .fold(0, |a, b| a | b);
    let words = quote!((0u8 #(+ ::sa_core::abi::stack_words::<#stack_tys>())*));
    // An `X87` result goes back with all 80 bits (`sa_core::abi::ST0_EXT`): the shim returns it as
    // f64 (ST0 = the value rounded to 53 bits) and the entry wrapper replaces ST0 with the full value.
    let x87_ret = matches!(&sig.output, ReturnType::Type(_, t)
        if quote!(#t).to_string().replace(' ', "").rsplit("::").next() == Some("X87"));
    let ext_lines: Vec<String> = if x87_ret {
        vec!["fstp st(0)".into(), "fld tbyte ptr [{ext}]".into()]
    } else {
        Vec::new()
    };
    let ext_op = if x87_ret {
        quote!(ext = sym ::sa_core::abi::ST0_EXT,)
    } else {
        quote!()
    };
    // the i686 stack word of each stack argument: the sum of the words before it. Pointer-sized
    // types are one word on i686 whatever the host's pointer size (the host build of the hook table
    // is what `sa-oracle` reads).
    let i686_words: Vec<_> = stack_tys
        .iter()
        .map(|t| {
            let s = quote!(#t).to_string().replace(' ', "");
            let ptr = ["GPtr<", "*", "&", "fn(", "Option<fn", "extern", "usize", "isize"]
                .iter()
                .any(|p| s.starts_with(p));
            if ptr {
                quote!(1u8)
            } else {
                quote!(::sa_core::abi::stack_words::<#t>())
            }
        })
        .collect();
    let offsets: Vec<_> = (0..stack_tys.len())
        .map(|i| {
            let before = &i686_words[..i];
            quote!((0u8 #(+ #before)*))
        })
        .collect();
    // a register convention registers its stack words as i686 counts on every target (the host
    // table's checks compare them with the exe's `ret n`)
    let reg_words = if regconv.is_some() { quote!((0u8 #(+ #i686_words)*)) } else { words.clone() };
    let arg8 = quote!((0u32 #(| ::sa_core::abi::narrow_bit::<#stack_tys>(#offsets, 1))*));
    let arg16 = quote!((0u32 #(| ::sa_core::abi::narrow_bit::<#stack_tys>(#offsets, 2))*));
    // The naked wrapper (entry when registers are preserved): push the saved registers, copy the
    // `n` stack argument words (each `push [esp + 4*(saved + n)]` copies the next lower word, the
    // displacement stays constant), call the shim (ECX/EDX still hold register arguments), drop the
    // copies for cdecl, pop, return as the original.
    // `entry_eax`: the wrapper first stores the caller's EAX in `sa_core::abi::ENTRY_EAX`.
    if footprint.is_some() && (!saved.is_empty() || entry_eax) {
        return Err(syn::Error::new(
            span,
            "footprint cannot be combined with preserve or entry_eax",
        ));
    }
    if peek.is_some() && (!saved.is_empty() || entry_eax || footprint.is_some()) {
        return Err(syn::Error::new(span, "peek cannot be combined with preserve, entry_eax or footprint"));
    }
    let regparts = match &regconv {
        Some((lit, spec)) => Some(reg_codegen(&RegIn {
            fname,
            args: &args,
            tys: &tys,
            ret: &ret,
            lit,
            spec,
            calls: &calls,
            shim: &shim,
            spec_const: &format_ident!("__SA_HOOK_REGCONV_{}", upper),
        })?),
        None => None,
    };
    let (wrapper, entry) = if let Some(rp) = &regparts {
        (rp.wrapper.clone(), rp.entry.clone())
    } else if let Some(np) = peek {
        // `peek = N` (`sa_core::abi::PEEK`): frame = PEEK.frames[depth] (layout: depth 0, saved EDI /
        // ESI / ECX 4..16, frames from 16, each [hook addr, entry ESP, N, N bytes]); EAX is free (no
        // convention passes an argument in it, and entry_eax is excluded). The window is copied before
        // anything is pushed; the arguments are copied as for `preserve`.
        let wrap = format_ident!("__sa_hook_peek_{}", fname);
        let mut l: Vec<String> = [
            "mov eax, dword ptr [{pk}]",
            "cmp eax, {dmax}",
            "jb 2f",
            "ud2",
            "2:",
            "mov dword ptr [{pk} + 4], edi",
            "mov dword ptr [{pk} + 8], esi",
            "mov dword ptr [{pk} + 12], ecx",
            "imul edi, eax, {fsz}",
            "lea edi, [{pk} + edi + 16]",
            "mov dword ptr [edi], {addr}",
            "mov dword ptr [edi + 4], esp",
            "mov dword ptr [edi + 8], {np}",
            "add edi, 12",
            "lea esi, [esp - {np}]",
            "mov ecx, {np4}",
            "rep movsd",
            "inc dword ptr [{pk}]",
            "mov edi, dword ptr [{pk} + 4]",
            "mov esi, dword ptr [{pk} + 8]",
            "mov ecx, dword ptr [{pk} + 12]",
            ".rept {n}",
            "push dword ptr [esp + {d}]",
            ".endr",
            "call {inner}",
        ]
        .map(String::from)
        .to_vec();
        if matches!(conv, Conv::Cdecl) {
            l.push("add esp, {b}".into());
        }
        l.push("dec dword ptr [{pk}]".into());
        l.push(if matches!(conv, Conv::Cdecl) {
            "ret".into()
        } else {
            "ret {b}".into()
        });
        let wrapper = quote! {
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[unsafe(naked)]
            #[allow(non_snake_case, unsafe_code, unused_variables, clippy::not_unsafe_ptr_arg_deref)]
            unsafe extern #abi fn #wrap(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                ::core::arch::naked_asm!(
                    #(#l),*,
                    n = const #words as u32,
                    d = const 4 * (#words as u32),
                    b = const 4 * (#words as u32),
                    np = const #np,
                    np4 = const #np / 4,
                    addr = const #addr,
                    dmax = const ::sa_core::abi::PEEK_DEPTH as u32,
                    fsz = const ::core::mem::size_of::<::sa_core::abi::PeekFrame>() as u32,
                    inner = sym #shim,
                    pk = sym ::sa_core::abi::PEEK,
                )
            }
        };
        (wrapper, quote!(#wrap as usize))
    } else if let Some(w) = footprint {
        // `footprint = W` (ADR-001 §5b, `sa_core::abi::stack_footprint`): snapshot the W bytes below
        // the entry ESP, copy the stack arguments, call the shim, put the snapshot back, apply the
        // port's recorded writes relative to the entry ESP, return as the original. EAX/EDX/ECX and
        // the x87 stack pass through (kept in the state's scratch words meanwhile).
        let wrap = format_ident!("__sa_hook_footprint_{}", fname);
        let mut l: Vec<String> = [
            "mov dword ptr [{fp}], 0",
            "mov dword ptr [{fp} + 4], 1",
            // the entry ESP (`sa_core::abi::footprint_entry_esp`, SAVE-1: locals of `and esp, -8` frames)
            "mov dword ptr [{fp} + {espo}], esp",
            // this state becomes the innermost (EAX is not an argument of a footprint hook)
            "mov eax, dword ptr [{cur}]",
            "mov dword ptr [{fp} + 28], eax",
            "lea eax, [{fp}]",
            "mov dword ptr [{cur}], eax",
            // snapshot [S - W, S) (ECX/EDX may carry register arguments: kept meanwhile)
            "mov dword ptr [{fp} + 16], ecx",
            "mov dword ptr [{fp} + 20], esi",
            "mov dword ptr [{fp} + 24], edi",
            "lea esi, [esp - {w}]",
            "lea edi, [{fp} + {snap}]",
            "mov ecx, {w4}",
            "rep movsd",
            "mov ecx, dword ptr [{fp} + 16]",
            "mov esi, dword ptr [{fp} + 20]",
            "mov edi, dword ptr [{fp} + 24]",
            ".rept {n}",
            "push dword ptr [esp + {d}]",
            ".endr",
            "call {inner}",
        ]
        .map(String::from)
        .to_vec();
        if matches!(conv, Conv::Cdecl) {
            l.push("add esp, {b}".into());
        }
        // ESP is the entry ESP again
        l.extend(
            [
                "mov dword ptr [{fp} + 8], eax",
                "mov dword ptr [{fp} + 12], edx",
                "mov dword ptr [{fp} + 16], ecx",
                "mov dword ptr [{fp} + 20], esi",
                "mov dword ptr [{fp} + 24], edi",
                "mov dword ptr [{fp} + 4], 0",
                "lea esi, [{fp} + {snap}]",
                "lea edi, [esp - {w}]",
                "mov ecx, {w4}",
                "rep movsd",
                "mov esi, dword ptr [{fp} + 20]",
                "mov edi, dword ptr [{fp} + 24]",
                "xor ecx, ecx",
                "2:",
                "cmp ecx, dword ptr [{fp}]",
                "jae 9f",
                "lea eax, [ecx + ecx*2]",
                "mov edx, dword ptr [{fp} + eax*4 + 40]",
                "cmp dword ptr [{fp} + eax*4 + 36], 1",
                "jne 3f",
                "mov edx, esi",
                "3:",
                "cmp dword ptr [{fp} + eax*4 + 36], 2",
                "jne 4f",
                "mov edx, edi",
                "4:",
                "cmp dword ptr [{fp} + eax*4 + 36], 3",
                "jne 5f",
                "mov edx, ebx",
                "5:",
                "cmp dword ptr [{fp} + eax*4 + 36], 4",
                "jne 6f",
                "mov edx, ebp",
                "6:",
                "cmp dword ptr [{fp} + eax*4 + 36], 5",
                "jne 7f",
                "add edx, esp",
                "7:",
                "cmp dword ptr [{fp} + eax*4 + 36], 6",
                "jne 8f",
                "mov edx, dword ptr fs:[0]",
                "8:",
                "mov eax, dword ptr [{fp} + eax*4 + 32]",
                "mov dword ptr [esp + eax], edx",
                "inc ecx",
                "jmp 2b",
                "9:",
                "mov dword ptr [{fp}], 0",
                "mov eax, dword ptr [{fp} + 28]",
                "mov dword ptr [{cur}], eax",
                "mov eax, dword ptr [{fp} + 8]",
                "mov edx, dword ptr [{fp} + 12]",
                "mov ecx, dword ptr [{fp} + 16]",
            ]
            .map(String::from),
        );
        l.extend(ext_lines.iter().cloned());
        l.push(if matches!(conv, Conv::Cdecl) {
            "ret".into()
        } else {
            "ret {b}".into()
        });
        let state = format_ident!("__SA_HOOK_FOOTPRINT_{}", fname.to_string().to_uppercase());
        let wrapper = quote! {
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static #state: ::sa_core::abi::FootprintCell = ::sa_core::abi::FootprintCell::new(#w);
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[unsafe(naked)]
            #[allow(non_snake_case, unsafe_code, unused_variables, dead_code, clippy::not_unsafe_ptr_arg_deref)]
            unsafe extern #abi fn #wrap(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                ::core::arch::naked_asm!(
                    #(#l),*,
                    n = const #words as u32,
                    d = const 4 * (#words as u32),
                    b = const 4 * (#words as u32),
                    w = const #w,
                    w4 = const #w / 4,
                    inner = sym #shim,
                    fp = sym #state,
                    cur = sym ::sa_core::abi::FOOTPRINT_CUR,
                    snap = const ::sa_core::abi::FOOTPRINT_SNAPSHOT_OFF,
                    espo = const ::sa_core::abi::FOOTPRINT_ENTRY_ESP_OFF,
                    #ext_op
                )
            }
        };
        (wrapper, quote!(#wrap as usize))
    } else if saved.is_empty() && entry_eax && !x87_ret {
        let wrap = format_ident!("__sa_hook_entry_eax_{}", fname);
        let wrapper = quote! {
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[unsafe(naked)]
            #[allow(non_snake_case, unsafe_code, unused_variables, dead_code, clippy::not_unsafe_ptr_arg_deref)]
            unsafe extern #abi fn #wrap(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                ::core::arch::naked_asm!(
                    "mov dword ptr [{slot}], eax",
                    "jmp {inner}",
                    slot = sym ::sa_core::abi::ENTRY_EAX,
                    inner = sym #shim,
                )
            }
        };
        (wrapper, quote!(#wrap as usize))
    } else if saved.is_empty() && !x87_ret {
        (quote!(), quote!(#shim as usize))
    } else {
        let wrap = format_ident!("__sa_hook_preserve_{}", fname);
        let mut lines: Vec<String> = Vec::new();
        if entry_eax {
            lines.push("mov dword ptr [{slot}], eax".into());
        }
        lines.extend(saved.iter().map(|r| format!("push {r}")));
        lines.push(".rept {n}".into());
        lines.push("push dword ptr [esp + {d}]".into());
        lines.push(".endr".into());
        lines.push("call {inner}".into());
        if matches!(conv, Conv::Cdecl) {
            lines.push("add esp, {b}".into());
        }
        lines.extend(ext_lines.iter().cloned());
        lines.extend(saved.iter().rev().map(|r| format!("pop {r}")));
        lines.push(if matches!(conv, Conv::Cdecl) {
            "ret".into()
        } else {
            "ret {b}".into()
        });
        let nsaved = saved.len() as u32;
        let slot_op = if entry_eax {
            quote!(slot = sym::sa_core::abi::ENTRY_EAX,)
        } else {
            quote!()
        };
        let wrapper = quote! {
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[unsafe(naked)]
            #[allow(non_snake_case, unsafe_code, unused_variables, dead_code, clippy::not_unsafe_ptr_arg_deref)]
            unsafe extern #abi fn #wrap(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                ::core::arch::naked_asm!(
                    #(#lines),*,
                    n = const #words as u32,
                    d = const 4 * (#nsaved + #words as u32),
                    b = const 4 * (#words as u32),
                    inner = sym #shim,
                    #slot_op
                    #ext_op
                )
            }
        };
        (wrapper, quote!(#wrap as usize))
    };
    // `fallback` (ADR-002): a cell the installer fills with the original entry (trampoline), and
    // `<fn>_original(args)` calling it with the original convention under the game's MXCSR
    // (`__orig_body!`, i.e. `call_original`). Phases B/C: the `unported` panic (or a host stand-in
    // registered for `<module>::<fn>_original`).
    let fallback_check = if regconv.is_some() {
        // a register-convention fallback reads its result from the call frame (`FromRegFrame`)
        quote!()
    } else {
        quote! {
            // A fallback returns the original's raw result: only types whose ABI value is the
            // type itself (integers, pointers, `()`) are allowed.
            const _: fn(#ret) -> <#ret as ::sa_core::abi::IntoAbi>::Abi = |x| x;
        }
    };
    let (fallback_items, set_original) = if fallback {
        let cell = format_ident!("__SA_HOOK_ORIGINAL_{}", upper);
        let ofn = format_ident!("{}_original", fname);
        let conv_id = match &regconv {
            Some((lit, _)) => quote!(reg(#lit)),
            None => {
                let i = Ident::new(
                    match conv {
                        Conv::Cdecl => "cdecl",
                        Conv::Stdcall => "stdcall",
                        Conv::Thiscall => "thiscall",
                        Conv::Fastcall => "fastcall",
                    },
                    Span::call_site(),
                );
                quote!(#i)
            }
        };
        let odoc = format!(
            "Runs the original body of `{}` ({addr:#x}) for a path this port does not implement \
             (ADR-002 fallback). Phase A only; phases B/C panic.",
            name.value()
        );
        (
            quote! {
                #[doc(hidden)]
                #[allow(dead_code)]
                static #cell: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);

                #fallback_check

                #[doc = #odoc]
                #[allow(clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments, clippy::unused_unit)]
                pub fn #ofn(#(#args: #tys),*) -> #ret {
                    ::sa_core::__orig_body!(unported #ofn, #conv_id,
                        (::sa_core::orig::fallback_entry(&#cell, #name)), (#(#args: #tys),*) -> #ret)
                }
            },
            quote!(Some(|a: u32| #cell.store(a, ::core::sync::atomic::Ordering::Release))),
        )
    } else {
        (quote!(), quote!(None))
    };
    let reg_spec_const = format_ident!("__SA_HOOK_REGCONV_{}", upper);
    let (shim_def, ret_expr, preserve_expr, regs_expr) = match regparts {
        Some(rp) => (rp.shim_def, rp.ret_expr, quote!(#reg_spec_const.preserve_mask()), quote!(Some(#reg_spec_const))),
        None => (
            quote! {
                #[cfg(target_arch = "x86")]
                #[doc(hidden)]
                #[allow(non_snake_case, unsafe_code, dead_code, clippy::not_unsafe_ptr_arg_deref)]
                extern #abi fn #shim(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                    #calls.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
                    let _fpu = ::sa_core::abi::FpuGuard::enter();
                    #[allow(unused_unsafe)]
                    let r = unsafe { #fname(#(#args),*) };
                    ::sa_core::abi::IntoAbi::into_abi(r)
                }
            },
            quote!(<#ret as ::sa_core::abi::IntoAbi>::RET),
            quote!(#mask),
            quote!(None),
        ),
    };
    Ok(quote! {
        #f

        #fallback_items

        #[doc(hidden)]
        static #calls: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);

        #shim_def

        #wrapper

        // Phase B on 32-bit x86: the native entry `orig!`'s address registry calls
        // (`sa_core::orig::registry`): the original convention with unwinding, so a panic in the port
        // (an unported callee further down) reaches the Rust caller.
        #[cfg(all(target_arch = "x86", not(sa_phase_a)))]
        #[doc(hidden)]
        #[allow(non_snake_case, unsafe_code, clippy::not_unsafe_ptr_arg_deref)]
        extern #abi_unwind fn #native(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
            #calls.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
            #[allow(unused_unsafe)]
            let r = unsafe { #fname(#(#args),*) };
            ::sa_core::abi::IntoAbi::into_abi(r)
        }

        #[doc = #doc]
        pub const #reg: ::sa_core::abi::HookReg = ::sa_core::abi::HookReg {
            addr: #addr,
            name: #name,
            conv: ::sa_core::abi::Conv::#variant,
            expect: &[#(#expect),*],
            entry: || {
                #[cfg(all(target_arch = "x86", sa_phase_a))]
                {
                    #entry
                }
                #[cfg(all(target_arch = "x86", not(sa_phase_a)))]
                {
                    #native as usize
                }
                #[cfg(not(target_arch = "x86"))]
                {
                    0
                }
            },
            calls: || #calls.load(::core::sync::atomic::Ordering::Relaxed),
            stack_words: #reg_words,
            ret: #ret_expr,
            this_type: #this_type,
            this_size: #this_size,
            preserve: #preserve_expr,
            set_original: #set_original,
            arg8: #arg8,
            arg16: #arg16,
            batch: #batch,
            regs: #regs_expr,
        };
    })
}

/// `#[native(addr = 0x…, conv = cdecl, name = "…")]`: registers a port that cannot be hooked in
/// phase A (a lone `ret`, an entry a loop branches back into, a register argument, an entry another
/// hook owns) as the phase B entry of its original address, without a phase A shim.
///
/// The function is left unchanged. Next to it the macro emits `pub const NATIVE_<NAME>:
/// sa_core::abi::NativeReg` (address, catalogue name, the native entry): on 32-bit x86 outside
/// phase A an `extern "<conv>-unwind"` function with the original convention that `orig!`'s
/// address registry calls (`sa_core::orig::registry`), 0 elsewhere. The crate lists these
/// constants in its `hooks::NATIVES`; `sa_ports::NATIVES` collects every crate's list.
#[proc_macro_attribute]
pub fn native(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut addr: Option<u32> = None;
    let mut conv: Option<Conv> = None;
    let mut name: Option<LitStr> = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("addr") {
            let v: LitInt = meta.value()?.parse()?;
            addr = Some(v.base10_parse::<u32>()?);
        } else if meta.path.is_ident("conv") {
            let v: Ident = meta.value()?.parse()?;
            conv = Some(Conv::parse(&v)?);
        } else if meta.path.is_ident("name") {
            name = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("expected addr, conv or name"));
        }
        Ok(())
    });
    parse_macro_input!(attr with parser);
    let f = parse_macro_input!(item as ItemFn);
    match expand_native(f, addr, conv, name) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_native(
    f: ItemFn,
    addr: Option<u32>,
    conv: Option<Conv>,
    name: Option<LitStr>,
) -> syn::Result<proc_macro2::TokenStream> {
    let span = f.sig.ident.span();
    let missing = |what: &str| syn::Error::new(span, format!("#[native] needs `{what} = ...`"));
    let addr = addr.ok_or_else(|| missing("addr"))?;
    let conv = conv.ok_or_else(|| missing("conv"))?;
    let name = name.ok_or_else(|| missing("name"))?;
    let sig = &f.sig;
    if !sig.generics.params.is_empty() || sig.asyncness.is_some() || sig.variadic.is_some() {
        return Err(syn::Error::new(
            span,
            "#[native] functions cannot be generic, async or variadic",
        ));
    }
    let mut args = Vec::new();
    let mut tys = Vec::new();
    for a in &sig.inputs {
        let FnArg::Typed(t) = a else {
            return Err(syn::Error::new(
                span,
                "#[native] goes on free functions (pass `this` explicitly)",
            ));
        };
        let Pat::Ident(p) = &*t.pat else {
            return Err(syn::Error::new(
                span,
                "#[native] arguments must be plain identifiers",
            ));
        };
        args.push(p.ident.clone());
        tys.push((*t.ty).clone());
    }
    if matches!(conv, Conv::Thiscall) && args.is_empty() {
        return Err(syn::Error::new(
            span,
            "a thiscall function takes `this` as its first argument",
        ));
    }
    let ret = match &sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, t) => quote!(#t),
    };
    let fname = &sig.ident;
    let upper = fname.to_string().to_uppercase();
    let native = format_ident!("__sa_native_{}", fname);
    let reg = format_ident!("NATIVE_{}", upper);
    let abi_unwind = LitStr::new(&format!("{}-unwind", conv.abi().value()), Span::call_site());
    let doc = format!(
        "Phase B registration of `{}` ({addr:#x}, not hookable in phase A).",
        name.value()
    );
    Ok(quote! {
        #f

        #[cfg(all(target_arch = "x86", not(sa_phase_a)))]
        #[doc(hidden)]
        #[allow(non_snake_case, unsafe_code, clippy::not_unsafe_ptr_arg_deref)]
        extern #abi_unwind fn #native(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
            #[allow(unused_unsafe)]
            let r = unsafe { #fname(#(#args),*) };
            ::sa_core::abi::IntoAbi::into_abi(r)
        }

        #[doc = #doc]
        pub const #reg: ::sa_core::abi::NativeReg = ::sa_core::abi::NativeReg {
            addr: #addr,
            name: #name,
            entry: || {
                #[cfg(all(target_arch = "x86", not(sa_phase_a)))]
                {
                    #native as usize
                }
                #[cfg(not(all(target_arch = "x86", not(sa_phase_a))))]
                {
                    0
                }
            },
        };
    })
}

/// The writer/verifier hints of `#[hook(unverified = ...)]` (docs/verify/HOWTO.md).
#[derive(Default)]
struct Hints {
    batch: Option<LitStr>,
    route: Option<LitStr>,
    hazards: Option<LitStr>,
    recs: Option<LitStr>,
    note: Option<LitStr>,
}

/// `route` values.
const ROUTES: &[&str] = &["L1-pure", "L2", "split", "L3-only"];
/// `hazards` values (besides `none`).
const HAZARDS: &[&str] = &[
    "x87-transcendental",
    "stack-residue",
    "entry-protection-read",
    "preserved-regs",
    "ax-al-return",
    "uninit-locals",
    "heap-writes",
];

impl Hints {
    fn check(&self) -> syn::Result<()> {
        let err = |l: &LitStr, m: String| Err(syn::Error::new(l.span(), m));
        if let Some(b) = &self.batch {
            let v = b.value();
            let ok = v.bytes().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && v.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'.')
                && v != "all"
                && !v.starts_with("0x");
            if !ok {
                return err(b, format!("unverified: batch id `{v}` must match [a-z0-9][a-z0-9.-]* (not `all`, not 0x...)"));
            }
            if self.route.is_none() {
                return Err(syn::Error::new(b.span(), format!("an unverified hook needs `route = \"{}\"`", ROUTES.join("|"))));
            }
            if self.hazards.is_none() {
                return Err(syn::Error::new(b.span(), "an unverified hook needs `hazards = \"none\"` or a list of hazards"));
            }
        }
        if let Some(r) = &self.route
            && !ROUTES.contains(&r.value().as_str())
        {
            return err(r, format!("route must be one of {}", ROUTES.join(", ")));
        }
        if let Some(h) = &self.hazards {
            let v = h.value();
            let items: Vec<&str> = v.split(',').map(str::trim).collect();
            if v.trim() != "none" {
                for x in &items {
                    if !HAZARDS.contains(x) {
                        return err(h, format!("hazard `{x}`: expected `none` or a list of {}", HAZARDS.join(", ")));
                    }
                }
            }
        }
        if let Some(r) = &self.recs
            && r.value().split(',').any(|x| x.trim().is_empty())
        {
            return err(r, "recs: a comma list of recording names".into());
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ register conventions --------

/// `conv = reg("...")` (HARNESS-5): the macro's own parse of the spec, mirrored by
/// `sa_core::regconv::RegConv::parse` (a const assertion in the expansion compares the two).
/// Location codes as `sa_core::regconv::Loc`, ret codes as `RetMode`, keep bits EAX 1, ECX 2, EDX 4.
struct RegSpec {
    args: Vec<u8>,
    ret: u8,
    keep: u8,
    pop: bool,
}

const LOCS: [&str; 18] = [
    "stack", "eax", "ecx", "edx", "ebx", "esi", "edi", "ebp", "ax", "cx", "dx", "bx", "al", "cl", "dl", "bl", "si", "di",
];
/// 32-bit register of each location code (index into LOCS)
const LOC_REG32: [&str; 18] = [
    "", "eax", "ecx", "edx", "ebx", "esi", "edi", "ebp", "eax", "ecx", "edx", "ebx", "eax", "ecx", "edx", "ebx", "esi", "edi",
];

impl RegSpec {
    fn parse(l: &LitStr) -> syn::Result<RegSpec> {
        let err = |m: String| syn::Error::new(l.span(), format!("reg conv: {m}"));
        let mut r = RegSpec { args: Vec::new(), ret: 0, keep: 0, pop: false };
        let mut seen_args = false;
        let v = l.value();
        let words = |x: &str| -> Vec<String> {
            x.split(|c: char| c == ',' || c.is_whitespace()).filter(|w| !w.is_empty()).map(String::from).collect()
        };
        for clause in v.split(';').map(str::trim).filter(|c| !c.is_empty()) {
            match clause.split_once('=') {
                None if clause == "pop" => r.pop = true,
                Some((n, val)) if n.trim() == "args" => {
                    if seen_args {
                        return Err(err("`args` given twice".into()));
                    }
                    seen_args = true;
                    for w in words(val) {
                        let code = LOCS.iter().position(|x| *x == w).ok_or_else(|| {
                            err(format!("`{w}` is not stack or a register name ({})", LOCS.join(" ")))
                        })? as u8;
                        if code != 0
                            && r.args.iter().any(|&c| c != 0 && LOC_REG32[c as usize] == LOC_REG32[code as usize])
                        {
                            return Err(err(format!("two arguments in {}", LOC_REG32[code as usize])));
                        }
                        r.args.push(code);
                    }
                    if r.args.len() > 12 {
                        return Err(err("more than 12 arguments".into()));
                    }
                }
                Some((n, val)) if n.trim() == "ret" => {
                    r.ret = match val.trim() {
                        "auto" => 0,
                        "al" => 1,
                        "ax" => 2,
                        "al:entry" => 3,
                        "ax:entry" => 4,
                        x => return Err(err(format!("ret `{x}`: expected auto, al, ax, al:entry or ax:entry"))),
                    }
                }
                Some((n, val)) if n.trim() == "keep" => {
                    for w in words(val) {
                        let bit = match w.as_str() {
                            "eax" => 1,
                            "ecx" => 2,
                            "edx" => 4,
                            _ => {
                                return Err(err(format!(
                                    "keep `{w}`: eax, ecx or edx (EBX/ESI/EDI/EBP are always kept)"
                                )));
                            }
                        };
                        if r.keep & bit != 0 {
                            return Err(err(format!("keep `{w}` twice")));
                        }
                        r.keep |= bit;
                    }
                    if r.keep == 0 {
                        return Err(err("keep without a register".into()));
                    }
                }
                _ => return Err(err(format!("unknown clause `{clause}` (args = ..., ret = ..., keep = ..., pop)"))),
            }
        }
        if r.keep & 1 != 0 && r.ret != 0 {
            return Err(err("keep = eax conflicts with a register result".into()));
        }
        Ok(r)
    }
    /// Location code of argument i (stack past the list).
    fn loc(&self, i: usize) -> u8 {
        self.args.get(i).copied().unwrap_or(0)
    }
}

struct RegIn<'a> {
    fname: &'a Ident,
    args: &'a [Ident],
    tys: &'a [syn::Type],
    ret: &'a proc_macro2::TokenStream,
    lit: &'a LitStr,
    spec: &'a RegSpec,
    calls: &'a Ident,
    shim: &'a Ident,
    spec_const: &'a Ident,
}

struct RegParts {
    shim_def: proc_macro2::TokenStream,
    wrapper: proc_macro2::TokenStream,
    entry: proc_macro2::TokenStream,
    ret_expr: proc_macro2::TokenStream,
}

/// The phase A side of a register-convention hook (`sa_core::regconv`): an `extern "C"` shim
/// `(out: *mut RegOut, register arguments as u32 words..., stack arguments with their types...)` and a
/// naked entry that builds that call from the original's registers and stack:
///
/// ```text
/// push ebp; mov ebp, esp                ; [ebp+8] = the original's first stack word
/// push ecx / push edx                   ; keep = ecx / edx
/// push esi; push edi                    ; used by the copy below
/// sub esp, 20; and esp, -16             ; RegOut at ESP (16-aligned), EAX/EDX = the caller's
/// mov [esp], eax; mov [esp+4], edx; mov byte [esp+18], 0
/// sub esp, F                            ; F = 4 * (1 + registers + stack words), rounded up to 16
/// mov [esp+4+4k], <register k>          ; register arguments in order (EBP's value from [ebp])
/// lea eax, [esp+F]; mov [esp], eax      ; the RegOut pointer
/// lea esi, [ebp+8]; lea edi, [esp+4+4*registers]; mov ecx, W; rep movsd   ; the stack arguments
/// call shim
/// add esp, F; mov eax, [esp]; mov edx, [esp+4]
/// cmp byte [esp+18], 0; je 2f; fld tbyte [esp+8]; 2:
/// lea esp, [ebp - 4*(kept + 2)]; pop edi; pop esi; pop edx / ecx; pop ebp
/// ret / ret 4*W                         ; `pop`
/// ```
fn reg_codegen(r: &RegIn) -> syn::Result<RegParts> {
    let RegIn { fname, args, tys, ret, lit, spec, calls, shim, spec_const } = *r;
    let mut reg_idx = Vec::new(); // (arg index, loc code)
    let mut stack_idx = Vec::new();
    for i in 0..args.len() {
        match spec.loc(i) {
            0 => stack_idx.push(i),
            c => reg_idx.push((i, c)),
        }
    }
    let nreg = reg_idx.len() as u32;
    let stack_tys: Vec<&syn::Type> = stack_idx.iter().map(|&i| &tys[i]).collect();
    let words = quote!((0u32 #(+ ::sa_core::abi::stack_words::<#stack_tys>() as u32)*));
    // shim parameters: registers as words, then the stack arguments with their types
    let mut params = Vec::new();
    for &(i, _) in &reg_idx {
        let a = &args[i];
        params.push(quote!(#a: u32));
    }
    for &i in &stack_idx {
        let (a, t) = (&args[i], &tys[i]);
        params.push(quote!(#a: #t));
    }
    let call_args: Vec<_> = (0..args.len())
        .map(|i| {
            let (a, t) = (&args[i], &tys[i]);
            if spec.loc(i) == 0 {
                quote!(#a)
            } else {
                quote!(<#t as ::sa_core::regconv::FromRegWord>::from_reg_word(#a))
            }
        })
        .collect();
    let checks: Vec<_> = (0..args.len())
        .map(|i| {
            let t = &tys[i];
            quote!(::sa_core::regconv::check_arg(&#spec_const, #i, ::core::mem::size_of::<#t>());)
        })
        .collect();
    let codes = &spec.args;
    let (sret, skeep, spop) = (spec.ret, spec.keep, spec.pop);
    let shim_def = quote! {
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const #spec_const: ::sa_core::regconv::RegConv = ::sa_core::regconv::RegConv::parse(#lit);
        const _: () = {
            assert!(
                #spec_const.same(&[#(#codes),*], #sret, #skeep, #spop),
                "#[hook] and sa_core::regconv parse this reg conv differently"
            );
            #(#checks)*
            ::sa_core::regconv::check_ret(&#spec_const, ::core::mem::size_of::<#ret>());
        };

        #[cfg(target_arch = "x86")]
        #[doc(hidden)]
        #[allow(non_snake_case, unsafe_code, clippy::not_unsafe_ptr_arg_deref, clippy::too_many_arguments)]
        extern "C" fn #shim(__sa_out: *mut ::sa_core::regconv::RegOut, #(#params),*) {
            #calls.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
            let _fpu = ::sa_core::abi::FpuGuard::enter();
            #[allow(unused_unsafe)]
            let r = unsafe { #fname(#(#call_args),*) };
            // SAFETY: the naked entry passes its RegOut slot.
            ::sa_core::regconv::RegReturn::put(r, unsafe { &mut *__sa_out }, #spec_const.ret_low_bytes());
        }
    };
    let mut kept: Vec<&str> = Vec::new();
    if spec.keep & 2 != 0 {
        kept.push("ecx");
    }
    if spec.keep & 4 != 0 {
        kept.push("edx");
    }
    let mut l: Vec<String> = vec!["push ebp".into(), "mov ebp, esp".into()];
    l.extend(kept.iter().map(|k| format!("push {k}")));
    l.extend(
        [
            "push esi",
            "push edi",
            "sub esp, 20",
            "and esp, -16",
            "mov dword ptr [esp], eax",
            "mov dword ptr [esp + 4], edx",
            "mov byte ptr [esp + 18], 0",
            "sub esp, {frame}",
        ]
        .map(String::from),
    );
    let mut ebp_slot = None;
    for (k, &(_, code)) in reg_idx.iter().enumerate() {
        let r32 = LOC_REG32[code as usize];
        let off = 4 + 4 * k;
        if r32 == "ebp" {
            ebp_slot = Some(off);
        } else {
            l.push(format!("mov dword ptr [esp + {off}], {r32}"));
        }
    }
    if let Some(off) = ebp_slot {
        l.push("mov esi, dword ptr [ebp]".into());
        l.push(format!("mov dword ptr [esp + {off}], esi"));
    }
    l.extend(["lea eax, [esp + {frame}]", "mov dword ptr [esp], eax", "lea esi, [ebp + 8]"].map(String::from));
    l.push(format!("lea edi, [esp + {}]", 4 + 4 * nreg));
    l.extend(
        [
            "mov ecx, {w}",
            "rep movsd",
            "call {inner}",
            "add esp, {frame}",
            "mov eax, dword ptr [esp]",
            "mov edx, dword ptr [esp + 4]",
            "cmp byte ptr [esp + 18], 0",
            "je 2f",
            "fld tbyte ptr [esp + 8]",
            "2:",
        ]
        .map(String::from),
    );
    l.push(format!("lea esp, [ebp - {}]", 4 * (kept.len() + 2)));
    l.push("pop edi".into());
    l.push("pop esi".into());
    l.extend(kept.iter().rev().map(|k| format!("pop {k}")));
    l.push("pop ebp".into());
    l.push(if spec.pop { "ret {b}".into() } else { "ret".into() });
    let wrap = format_ident!("__sa_hook_reg_{}", fname);
    let b_op = if spec.pop { quote!(b = const 4 * (#words),) } else { quote!() };
    let wrapper = quote! {
        #[cfg(target_arch = "x86")]
        #[doc(hidden)]
        #[unsafe(naked)]
        #[allow(non_snake_case, unsafe_code)]
        unsafe extern "C" fn #wrap() {
            ::core::arch::naked_asm!(
                #(#l),*,
                frame = const (4 * (1 + #nreg + #words) + 15) & !15,
                w = const #words,
                #b_op
                inner = sym #shim,
            )
        }
    };
    Ok(RegParts {
        shim_def,
        wrapper,
        entry: quote!(#wrap as usize),
        ret_expr: quote!(#spec_const.abi_ret(<#ret as ::sa_core::regconv::RegReturn>::RET)),
    })
}
