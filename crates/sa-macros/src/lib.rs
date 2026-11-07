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
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("addr") {
            let v: LitInt = meta.value()?.parse()?;
            addr = Some(v.base10_parse::<u32>()?);
        } else if meta.path.is_ident("conv") {
            let v: Ident = meta.value()?.parse()?;
            conv = Some(Conv::parse(&v)?);
        } else if meta.path.is_ident("name") {
            name = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("expect") {
            let v: LitStr = meta.value()?.parse()?;
            expect = Some(parse_expect(&v)?);
        } else if meta.path.is_ident("this") {
            this = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("preserve") {
            preserve = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("expected addr, conv, name, expect, this or preserve"));
        }
        Ok(())
    });
    parse_macro_input!(attr with parser);
    let f = parse_macro_input!(item as ItemFn);
    match expand(f, addr, conv, name, expect, this, preserve) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(
    f: ItemFn,
    addr: Option<u32>,
    conv: Option<Conv>,
    name: Option<LitStr>,
    expect: Option<Vec<u8>>,
    this: Option<LitStr>,
    preserve: Option<LitStr>,
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
    let stack_tys: Vec<_> = tys.iter().skip(in_regs).collect();
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
    let variant = conv.variant();
    let doc = format!("Hook registration of `{}` ({addr:#x}).", name.value());
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
    // The naked wrapper (entry when registers are preserved): push the saved registers, copy the
    // `n` stack argument words (each `push [esp + 4*(saved + n)]` copies the next lower word, the
    // displacement stays constant), call the shim (ECX/EDX still hold register arguments), drop the
    // copies for cdecl, pop, return as the original.
    let (wrapper, entry) = if saved.is_empty() {
        (quote!(), quote!(#shim as usize))
    } else {
        let wrap = format_ident!("__sa_hook_preserve_{}", fname);
        let mut lines: Vec<String> = saved.iter().map(|r| format!("push {r}")).collect();
        lines.push(".rept {n}".into());
        lines.push("push dword ptr [esp + {d}]".into());
        lines.push(".endr".into());
        lines.push("call {inner}".into());
        if matches!(conv, Conv::Cdecl) {
            lines.push("add esp, {b}".into());
        }
        lines.extend(saved.iter().rev().map(|r| format!("pop {r}")));
        lines.push(if matches!(conv, Conv::Cdecl) { "ret".into() } else { "ret {b}".into() });
        let nsaved = saved.len() as u32;
        let wrapper = quote! {
            #[cfg(target_arch = "x86")]
            #[doc(hidden)]
            #[unsafe(naked)]
            #[allow(non_snake_case, unsafe_code, unused_variables, clippy::not_unsafe_ptr_arg_deref)]
            unsafe extern #abi fn #wrap(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
                ::core::arch::naked_asm!(
                    #(#lines),*,
                    n = const #words as u32,
                    d = const 4 * (#nsaved + #words as u32),
                    b = const 4 * (#words as u32),
                    inner = sym #shim,
                )
            }
        };
        (wrapper, quote!(#wrap as usize))
    };
    Ok(quote! {
        #f

        #[doc(hidden)]
        static #calls: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);

        #[cfg(target_arch = "x86")]
        #[doc(hidden)]
        #[allow(non_snake_case, unsafe_code, clippy::not_unsafe_ptr_arg_deref)]
        extern #abi fn #shim(#(#args: #tys),*) -> <#ret as ::sa_core::abi::IntoAbi>::Abi {
            #calls.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
            let _fpu = ::sa_core::abi::FpuGuard::enter();
            #[allow(unused_unsafe)]
            let r = unsafe { #fname(#(#args),*) };
            ::sa_core::abi::IntoAbi::into_abi(r)
        }

        #wrapper

        #[doc = #doc]
        pub const #reg: ::sa_core::abi::HookReg = ::sa_core::abi::HookReg {
            addr: #addr,
            name: #name,
            conv: ::sa_core::abi::Conv::#variant,
            expect: &[#(#expect),*],
            entry: || {
                #[cfg(target_arch = "x86")]
                {
                    #entry
                }
                #[cfg(not(target_arch = "x86"))]
                {
                    0
                }
            },
            calls: || #calls.load(::core::sync::atomic::Ordering::Relaxed),
            stack_words: #words,
            ret: <#ret as ::sa_core::abi::IntoAbi>::RET,
            this_type: #this_type,
            this_size: #this_size,
            preserve: #mask,
        };
    })
}
