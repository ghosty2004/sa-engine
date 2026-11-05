// CAT-14: decompile a list of functions of gta_sa.exe 1.0 US (spot checks of gta-reversed deviations).
//
// Run read-only on the analysed project in target/ghidra:
//   analyzeHeadless target/ghidra gta_sa_10us -process gta_sa.exe -readOnly -noanalysis \
//     -scriptPath tools/catalog/ghidra -postScript DecompileList.java <addrs.txt> <out.c>
// addrs.txt: one hex address per line. A function is created where none exists (not saved:
// -readOnly). A 5-byte `jmp` stub into .HOODLUM is followed to its body.
// out.c: "// ==== <addr> -> <body addr> <name>" followed by the decompiled C for each address.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;

import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;

public class DecompileList extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        List<String> lines = Files.readAllLines(Paths.get(args[0]));
        DecompInterface dec = new DecompInterface();
        dec.openProgram(currentProgram);
        try (PrintWriter out = new PrintWriter(args[1])) {
            for (String l : lines) {
                l = l.trim();
                if (l.isEmpty()) continue;
                Address a = toAddr(Long.parseLong(l.replace("0x", ""), 16));
                Address body = a;
                Instruction ins = getInstructionAt(a);
                if (ins == null) {
                    disassemble(a);
                    ins = getInstructionAt(a);
                }
                if (ins != null && ins.getMnemonicString().equalsIgnoreCase("JMP") && ins.getFlows().length == 1) {
                    Address t = ins.getFlows()[0];
                    if (t.getOffset() >= 0x1556000L) body = t;
                }
                Function f = getFunctionAt(body);
                if (f == null) {
                    if (getInstructionAt(body) == null) disassemble(body);
                    f = createFunction(body, null);
                }
                if (f == null) {
                    out.println("// ==== " + l + " -> " + body + " (no function)");
                    continue;
                }
                DecompileResults r = dec.decompileFunction(f, 120, monitor);
                out.println("// ==== " + l + " -> " + body + " " + f.getName());
                out.println(r.decompileCompleted() ? r.getDecompiledFunction().getC() : "// decompile failed: " + r.getErrorMessage());
            }
        }
        dec.dispose();
    }
}
