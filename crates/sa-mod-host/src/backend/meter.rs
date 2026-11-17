//! Fuel metering by instrumentation, for engines without fuel (the browser's WebAssembly): the module is rewritten
//! once at load time so that it counts down a mutable `i32` global, exported as [`FUEL_EXPORT`], and traps
//! (`unreachable`) when it goes negative. The glue sets the global before each call; a trap with a negative global
//! is a fuel overrun. The memory's maximum is lowered to the mod's cap, so `memory.grow` fails beyond it.
//!
//! Check points: every function entry and every `loop` header (so every endless loop and every endless recursion
//! passes one). Each subtracts the number of operators from it to the next check point in the body's linear order:
//! an upper bound of the straight-line work it guards, close to wasmtime's one unit per operator.

use wasm_encoder::reencode::{Error, Reencode, utils};
use wasm_encoder::{ConstExpr, ExportKind, GlobalType, Instruction, ValType};
use wasmparser::{Operator, Parser, Payload, TypeRef};

/// The export name of the fuel counter.
pub const FUEL_EXPORT: &str = "__sa_fuel";

struct Meter {
    /// The fuel global's index (after every imported and defined global).
    fuel: u32,
    /// The memory cap in pages.
    cap_pages: u64,
    /// The counter's initial value (the budget of a start function).
    initial: i32,
    /// The module has its own global section (else one is inserted).
    has_globals: bool,
    added_global: bool,
}

/// The section order of the binary format (custom sections excluded).
fn order(id: wasm_encoder::SectionId) -> u8 {
    use wasm_encoder::SectionId as S;
    match id {
        S::Type => 1,
        S::Import => 2,
        S::Function => 3,
        S::Table => 4,
        S::Memory => 5,
        S::Tag => 6,
        S::Global => 7,
        S::Export => 8,
        S::Start => 9,
        S::Element => 10,
        S::DataCount => 11,
        S::Code => 12,
        S::Data => 13,
        _ => 0,
    }
}

impl Meter {
    fn fuel_global(&mut self, globals: &mut wasm_encoder::GlobalSection) {
        globals.global(
            GlobalType { val_type: ValType::I32, mutable: true, shared: false },
            &ConstExpr::i32_const(self.initial),
        );
        self.added_global = true;
    }

    fn check(&self, f: &mut wasm_encoder::Function, cost: usize) {
        let cost = cost.clamp(1, i32::MAX as usize) as i32;
        f.instruction(&Instruction::GlobalGet(self.fuel));
        f.instruction(&Instruction::I32Const(cost));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::GlobalSet(self.fuel));
        f.instruction(&Instruction::GlobalGet(self.fuel));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Unreachable);
        f.instruction(&Instruction::End);
    }
}

impl Reencode for Meter {
    type Error = std::convert::Infallible;

    fn memory_type(&mut self, ty: wasmparser::MemoryType) -> Result<wasm_encoder::MemoryType, Error<Self::Error>> {
        let mut m = wasm_encoder::MemoryType::from(ty);
        m.maximum = Some(m.maximum.map_or(self.cap_pages, |x| x.min(self.cap_pages)));
        Ok(m)
    }

    fn parse_global_section(
        &mut self,
        globals: &mut wasm_encoder::GlobalSection,
        section: wasmparser::GlobalSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        utils::parse_global_section(self, globals, section)?;
        self.fuel_global(globals);
        Ok(())
    }

    fn intersperse_section_hook(
        &mut self,
        module: &mut wasm_encoder::Module,
        after: Option<wasm_encoder::SectionId>,
        before: Option<wasm_encoder::SectionId>,
    ) -> Result<(), Error<Self::Error>> {
        let g = order(wasm_encoder::SectionId::Global);
        if !self.has_globals && !self.added_global && after.is_none_or(|a| order(a) < g) && before.is_none_or(|b| order(b) > g)
        {
            let mut globals = wasm_encoder::GlobalSection::new();
            self.fuel_global(&mut globals);
            module.section(&globals);
        }
        Ok(())
    }

    fn parse_export_section(
        &mut self,
        exports: &mut wasm_encoder::ExportSection,
        section: wasmparser::ExportSectionReader<'_>,
    ) -> Result<(), Error<Self::Error>> {
        utils::parse_export_section(self, exports, section)?;
        exports.export(FUEL_EXPORT, ExportKind::Global, self.fuel);
        Ok(())
    }

    fn parse_function_body(
        &mut self,
        code: &mut wasm_encoder::CodeSection,
        func: wasmparser::FunctionBody<'_>,
    ) -> Result<(), Error<Self::Error>> {
        let mut f = self.new_function_with_parsed_locals(&func)?;
        let mut ops = Vec::new();
        let mut reader = func.get_operators_reader()?;
        while !reader.eof() {
            ops.push(reader.read()?);
        }
        // the operators guarded by the check point before `ops[i]`: up to the next loop header
        let segment = |i: usize| ops[i..].iter().position(|o| matches!(o, Operator::Loop { .. })).unwrap_or(ops.len() - i);
        self.check(&mut f, segment(0));
        for (i, op) in ops.iter().enumerate() {
            let is_loop = matches!(op, Operator::Loop { .. });
            f.instruction(&self.instruction(op.clone())?);
            if is_loop {
                self.check(&mut f, segment(i + 1) + 1);
            }
        }
        code.function(&f);
        Ok(())
    }
}

/// Instruments `wasm` (a module that passed [`crate::module::check`]) with the fuel counter (starting at `initial`)
/// and the memory cap.
pub fn instrument(wasm: &[u8], memory_bytes: u64, initial: i32) -> Result<Vec<u8>, String> {
    let (mut imported, mut defined, mut has_globals) = (0u32, 0u32, false);
    for p in Parser::new(0).parse_all(wasm) {
        match p.map_err(|e| e.to_string())? {
            Payload::ImportSection(r) => {
                for i in r.into_imports() {
                    if matches!(i.map_err(|e| e.to_string())?.ty, TypeRef::Global(_)) {
                        imported += 1;
                    }
                }
            }
            Payload::GlobalSection(r) => {
                defined = r.count();
                has_globals = true;
            }
            _ => {}
        }
    }
    let mut meter = Meter {
        fuel: imported + defined,
        cap_pages: memory_bytes / crate::module::PAGE,
        initial,
        has_globals,
        added_global: false,
    };
    let mut module = wasm_encoder::Module::new();
    meter
        .parse_core_module(&mut module, Parser::new(0), wasm)
        .map_err(|e| format!("instrument: {e}"))?;
    Ok(module.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::tests::wat_mod;
    use wasmtime::{Engine, Linker, Module, Store, Val};

    /// Runs `sa_overlay` of the instrumented `wasm` with `fuel` (wasmtime without its own fuel); returns whether it
    /// trapped and the counter afterwards.
    fn run(wasm: &[u8], fuel: i32) -> (bool, i32) {
        let out = instrument(wasm, 1 << 20, 0).unwrap();
        wasmparser::Validator::new().validate_all(&out).unwrap();
        let engine = Engine::default();
        let module = Module::new(&engine, &out).unwrap();
        let mut store = Store::new(&engine, ());
        let mut linker = Linker::new(&engine);
        linker.func_wrap("sa", "overlay_rect", |_: f32, _: f32, _: f32, _: f32, _: i32| 0i32).unwrap();
        let inst = linker.instantiate(&mut store, &module).unwrap();
        let g = inst.get_global(&mut store, FUEL_EXPORT).unwrap();
        g.set(&mut store, Val::I32(fuel)).unwrap();
        let mem = inst.get_memory(&mut store, "memory").unwrap();
        assert_eq!(mem.ty(&store).maximum(), Some(16));
        let f = inst.get_typed_func::<i32, i32>(&mut store, "sa_overlay").unwrap();
        let trapped = f.call(&mut store, 0).is_err();
        (trapped, g.get(&mut store).i32().unwrap())
    }

    #[test]
    fn counts_and_stops_loops() {
        let (trapped, left) = run(&wat_mod("", "1", "i32.const 0"), 1000);
        assert!(!trapped);
        assert!(left < 1000 && left > 990, "{left}");
        let (trapped, left) = run(&wat_mod("", "1 64", "(loop $l (br $l)) i32.const 0"), 100_000);
        assert!(trapped && left < 0);
        // a module without a global section of its own gets one
        let out = instrument(&wat::parse_str(r#"(module (func (export "f") (loop $l (br_if $l (i32.const 1)))))"#).unwrap(), 1 << 20, 0)
            .unwrap();
        wasmparser::Validator::new().validate_all(&out).unwrap();
    }
}
