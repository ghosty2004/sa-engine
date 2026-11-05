// CAT-7: dump function bodies and flow references of gta_sa.exe 1.0 US as JSON lines.
//
// Run by tools/catalog/callgraph.py (subcommand `ghidra`) on the auto-analysed project in
// target/ghidra, always with -readOnly so every run starts from the same analysed state.
//
// Script arguments:
//   0  remove.txt  one hex address per line: procedures that are not functions (CAT-5
//                  excluded:mis-split and excluded:protection). Their Function objects are
//                  removed first, so host bodies flow through displaced-code fragments and past
//                  SecuROM re-entry points.
//   1  create.txt  one hex address per line: every catalogue procedure plus each .HOODLUM body
//                  reached from a game-.text stub. A function is created at each one that has none
//                  (this covers the 1,888 gta-reversed hook targets Ghidra's analysis missed).
//   2  out.jsonl   output, one JSON object per function whose entry lies in [lo, hi) of the
//                  game .text, _rwcseg or .HOODLUM.
//
// Steps, in order (each counted in the closing "CAT7 ..." log line):
//   1. remove the not-a-function procedures; drop flow overrides on jumps into them
//   2. clear the non-returning flag Ghidra put on _free and its thunks (keeps it on abort, exit,
//      _CxxThrowException, longjmp) and restore the fall-through of their call sites
//   3. create functions at every create.txt address (clearing a wrongly typed data unit if needed)
//   4. un-thunk functions whose thunk target is a removed fragment
//   5. heal: disassemble branch targets / fall-throughs of function code left undecoded, also
//      clearing misaligned instructions (from bogus flows) that are not in a catalogue function
//   6. recompute every body with CreateFunctionCmd.getFunctionBody (twice, address order) and
//      dump the flow-derived body even where setBody refuses it (overlapping bodies)
// Output per function:
//   {"entry":"0x..","name":"..","thunk":"0x..|null","created":bool,
//    "body":[["0xlo","0xhi"],...]   (inclusive ranges, address order)
//    "calls":[["0xsite","0xto","TYPE"],...]  every CALL-type reference from the body
//    "jumps":[["0xsite","0xto","TYPE"],...]  computed-jump references (switch tables)
//    "unres":[["0xsite","kind"],...]  computed call/jump instructions with no reference}
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;

import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;

public class DumpCat7 extends GhidraScript {

    private static final long[][] DUMP_RANGES = {
        {0x401000L, 0x858000L},    // game .text + _rwcseg
        {0x1556000L, 0x1577000L},  // .HOODLUM
    };

    private List<Address> readAddrs(String path) throws IOException {
        List<Address> out = new ArrayList<>();
        for (String line : Files.readAllLines(Paths.get(path), StandardCharsets.UTF_8)) {
            line = line.trim();
            if (line.isEmpty()) continue;
            out.add(toAddr(Long.parseLong(line.replace("0x", ""), 16)));
        }
        Collections.sort(out);
        return out;
    }

    private static boolean inDump(long a) {
        for (long[] r : DUMP_RANGES) if (a >= r[0] && a < r[1]) return true;
        return false;
    }

    private static String hx(Address a) {
        return "\"0x" + Long.toHexString(a.getOffset()) + "\"";
    }

    private static String js(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            if (c == '"' || c == '\\') b.append('\\').append(c);
            else if (c < 0x20) b.append(String.format("\\u%04x", (int) c));
            else b.append(c);
        }
        return b.append('"').toString();
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        List<Address> remove = readAddrs(args[0]);
        List<Address> create = readAddrs(args[1]);
        FunctionManager fm = currentProgram.getFunctionManager();
        Listing listing = currentProgram.getListing();

        int removed = 0;
        for (Address a : remove) {
            if (fm.getFunctionAt(a) != null) {
                fm.removeFunction(a);
                removed++;
            }
        }
        // Auto-analysis turned some `jmp fragment` instructions into calls (flow override
        // CALL_RETURN) because the fragment was a function. The fragment is code of the host,
        // so drop the override on every jump into a removed procedure.
        int overrides = 0;
        ReferenceManager rm = currentProgram.getReferenceManager();
        for (Address a : remove) {
            List<Address> srcs = new ArrayList<>();
            for (Reference r : rm.getReferencesTo(a)) srcs.add(r.getFromAddress());
            for (Address s : srcs) {
                Instruction ins = listing.getInstructionAt(s);
                if (ins != null && ins.getFlowOverride() != FlowOverride.NONE) {
                    ins.setFlowOverride(FlowOverride.NONE);
                    overrides++;
                }
            }
        }
        // Non-returning functions: auto-analysis flagged _free (and the thunks operator delete
        // etc. that jump to it) as non-returning, which cuts every caller after the call. Keep
        // the flag only on the genuinely non-returning MSVC runtime functions named below and
        // restore the fall-through of every call site of the others.
        Set<String> realNoReturn = new HashSet<>(Arrays.asList(
            "_abort", "__exit", "_exit", "__CxxThrowException@8", "_longjmp", "__amsg_exit", "_ExitProcess"));
        int noretCleared = 0, callsitesFixed = 0;
        List<Function> noret = new ArrayList<>();
        for (Function f : fm.getFunctions(true)) {
            if (!f.hasNoReturn()) continue;
            Function base = f.isThunk() ? f.getThunkedFunction(true) : f;
            if (base != null && realNoReturn.contains(base.getName())) continue;
            noret.add(f);
        }
        for (Function f : noret) {
            if (f.hasNoReturn()) f.setNoReturn(false);
            noretCleared++;
            println("CAT7 noreturn cleared " + f.getEntryPoint() + " " + f.getName());
            List<Address> srcs = new ArrayList<>();
            for (Reference r : rm.getReferencesTo(f.getEntryPoint())) srcs.add(r.getFromAddress());
            for (Address src : srcs) {
                Instruction ins = listing.getInstructionAt(src);
                if (ins == null || !ins.getFlowType().isCall()) continue;
                if (ins.getFlowOverride() == FlowOverride.CALL_RETURN && ins.getMnemonicString().equalsIgnoreCase("JMP")) {
                    continue;  // a tail jump: CALL_RETURN is right there
                }
                if (ins.getFlowOverride() != FlowOverride.NONE) ins.setFlowOverride(FlowOverride.NONE);
                Address ft = ins.getDefaultFallThrough();
                if (ft != null && listing.getInstructionAt(ft) == null) {
                    Data d = listing.getDataContaining(ft);
                    if (d == null || !d.isDefined()) disassemble(ft);
                }
                callsitesFixed++;
            }
        }
        Set<Address> created = new HashSet<>();
        int failed = 0, forced = 0;
        for (Address a : create) {
            if (fm.getFunctionAt(a) != null) continue;
            if (listing.getInstructionAt(a) == null) {
                if (listing.getDataContaining(a) != null) {
                    // undefined-type data only; real data at a catalogue entry is reported as a failure
                    Data d = listing.getDataContaining(a);
                    if (!d.isDefined()) clearListing(a);
                }
                disassemble(a);
            }
            Function f = createFunction(a, null);
            if (f == null) {
                // createFunction refuses entries inside another body; use the command directly
                CreateFunctionCmd cmd = new CreateFunctionCmd(null, a, null, SourceType.ANALYSIS);
                cmd.applyTo(currentProgram, monitor);
                f = fm.getFunctionAt(a);
            }
            if (f == null && listing.getDataContaining(a) != null) {
                // Auto-analysis typed bytes at a known entry as data (e.g. a pointer guess). The
                // entry is a verified catalogue function (gta-reversed hook, first instruction
                // checked by build_catalog.py), so clear the data units the flow from it runs
                // into, up to the next catalogue entry, and retry.
                int idx = create.indexOf(a);
                Address lim = idx + 1 < create.size() ? create.get(idx + 1) : a.add(0x1000);
                for (int tries = 0; tries < 64 && fm.getFunctionAt(a) == null; tries++) {
                    Data d = null;
                    for (Address p = a; p.compareTo(lim) < 0; ) {
                        CodeUnit cu = listing.getCodeUnitContaining(p);
                        if (cu == null) { p = p.add(1); continue; }
                        if (cu instanceof Data) { d = (Data) cu; break; }
                        p = cu.getMaxAddress().add(1);
                    }
                    if (d == null) break;
                    println("CAT7 cleared data " + d.getMinAddress() + " " + d + " for entry " + a);
                    clearListing(d.getMinAddress(), d.getMaxAddress());
                    forced++;
                    disassemble(a);
                    if (createFunction(a, null) == null) {
                        new CreateFunctionCmd(null, a, null, SourceType.ANALYSIS).applyTo(currentProgram, monitor);
                    }
                }
                f = fm.getFunctionAt(a);
            }
            if (f != null) created.add(a);
            else {
                failed++;
                CodeUnit cu = listing.getCodeUnitContaining(a);
                Function host = fm.getFunctionContaining(a);
                println("CAT7 create failed at " + a + " unit=" + (cu == null ? "none" : cu.getMinAddress() + " " + cu)
                        + " host=" + (host == null ? "none" : host.getEntryPoint().toString()));
            }
        }
        // Creating a function whose first instruction is `jmp fragment` makes Ghidra mark it a
        // thunk and re-create a function at the fragment. Undo both: the fragment belongs to
        // the host body.
        Set<Address> removeSet = new HashSet<>(remove);
        int unthunked = 0;
        for (Function f : fm.getFunctions(true)) {
            if (!f.isThunk() || !inDump(f.getEntryPoint().getOffset())) continue;
            Function t = f.getThunkedFunction(false);
            if (t != null && removeSet.contains(t.getEntryPoint())) {
                f.setThunkedFunction(null);
                unthunked++;
            }
        }
        for (Address a : remove) {
            if (fm.getFunctionAt(a) != null) {
                fm.removeFunction(a);
                removed++;
            }
        }
        // Recompute every body in the dump ranges, in address order, twice: the first pass
        // shrinks bodies that now run into newly created entries, the second makes the result
        // independent of the order of the first.
        List<Function> funcs = new ArrayList<>();
        for (Function f : fm.getFunctions(true)) {
            if (inDump(f.getEntryPoint().getOffset())) funcs.add(f);
        }
        // Heal: auto-analysis sometimes typed the target of a branch or a fall-through as data
        // (pointer/undefined4 guesses), which cuts the body there. Any such unit that a function
        // instruction flows into is cleared and disassembled. Repeat until nothing changes.
        int healed = 0, conflicts = 0;
        Set<Address> createSet = new HashSet<>(create);
        for (int round = 0; round < 8; round++) {
            int before = healed;
            for (Function f : funcs) {
                if (f.isThunk()) continue;
                AddressSetView body = CreateFunctionCmd.getFunctionBody(currentProgram, f.getEntryPoint(), monitor);
                if (body == null) continue;
                InstructionIterator it = listing.getInstructions(body, true);
                List<Address> targets = new ArrayList<>();
                while (it.hasNext()) {
                    Instruction ins = it.next();
                    for (Address t : ins.getFlows()) targets.add(t);
                    if (ins.getFallThrough() != null) targets.add(ins.getFallThrough());
                }
                for (Address t : targets) {
                    if (!inDump(t.getOffset()) || listing.getInstructionAt(t) != null) continue;
                    Data d = listing.getDataContaining(t);
                    if (d != null && d.isDefined()) {
                        println("CAT7 healed data " + d.getMinAddress() + " " + d + " (flow target " + t
                                + " from function " + f.getEntryPoint() + ")");
                        clearListing(d.getMinAddress(), d.getMaxAddress());
                    }
                    if (listing.getInstructionAt(t) == null) disassemble(t);
                    // A misaligned instruction from a bogus flow (e.g. a SecuROM data reference
                    // decoded as code) can block the real decode. Clear such instructions when
                    // they are not inside a catalogue function, then retry.
                    for (int k = 0; k < 4 && listing.getInstructionAt(t) == null; k++) {
                        boolean cleared = false;
                        for (int off = -15; off < 16; off++) {
                            Instruction c = listing.getInstructionContaining(t.add(off));
                            if (c == null || c.getMinAddress().equals(t)) continue;
                            if (c.getMaxAddress().compareTo(t) < 0) continue;
                            Function cf = fm.getFunctionContaining(c.getMinAddress());
                            if (cf != null && createSet.contains(cf.getEntryPoint())) continue;
                            println("CAT7 healed conflict " + c.getMinAddress() + " " + c + " (flow target " + t
                                    + " from function " + f.getEntryPoint() + ")");
                            clearListing(c.getMinAddress(), c.getMaxAddress());
                            conflicts++;
                            cleared = true;
                        }
                        if (!cleared) break;
                        disassemble(t);
                    }
                    if (listing.getInstructionAt(t) != null) healed++;
                }
            }
            if (healed == before) break;
        }
        int setBodyFailed = 0;
        // The flow-derived body is what gets dumped, even where Ghidra refuses to store it
        // (setBody rejects bodies that overlap another function's, e.g. shared tails).
        Map<Address, AddressSetView> bodies = new HashMap<>();
        for (int pass = 0; pass < 2; pass++) {
            for (Function f : funcs) {
                if (f.isThunk()) continue;
                // getFunctionBody follows flow from the entry and stops at other entries;
                // unlike fixupFunctionBody it never turns a one-jump function back into a thunk.
                AddressSetView body = CreateFunctionCmd.getFunctionBody(currentProgram, f.getEntryPoint(), monitor);
                if (body != null && !body.isEmpty()) bodies.put(f.getEntryPoint(), body);
                try {
                    if (body != null && !body.isEmpty()) f.setBody(body);
                } catch (Exception e) {
                    if (pass == 1) {
                        setBodyFailed++;
                        println("CAT7 setBody failed at " + f.getEntryPoint() + ": " + e.getMessage());
                    }
                }
            }
        }

        try (Writer w = new BufferedWriter(new OutputStreamWriter(
                new FileOutputStream(args[2]), StandardCharsets.UTF_8))) {
            for (Function f : funcs) {
                StringBuilder b = new StringBuilder();
                b.append("{\"entry\":").append(hx(f.getEntryPoint()));
                b.append(",\"name\":").append(js(f.getName()));
                Function t = f.isThunk() ? f.getThunkedFunction(false) : null;
                b.append(",\"thunk\":").append(t == null ? "null" : hx(t.getEntryPoint()));
                b.append(",\"created\":").append(created.contains(f.getEntryPoint()));
                b.append(",\"body\":[");
                boolean first = true;
                AddressSetView fbody = bodies.getOrDefault(f.getEntryPoint(), f.getBody());
                for (AddressRange r : fbody.getAddressRanges()) {
                    if (!first) b.append(',');
                    first = false;
                    b.append('[').append(hx(r.getMinAddress())).append(',').append(hx(r.getMaxAddress())).append(']');
                }
                b.append("]");
                StringBuilder calls = new StringBuilder(), jumps = new StringBuilder(), unres = new StringBuilder();
                InstructionIterator it = listing.getInstructions(fbody, true);
                while (it.hasNext()) {
                    Instruction ins = it.next();
                    boolean anyCall = false, anyJump = false;
                    for (Reference r : ins.getReferencesFrom()) {
                        RefType rt = r.getReferenceType();
                        if (rt.isCall()) {
                            anyCall = true;
                            if (calls.length() > 0) calls.append(',');
                            calls.append('[').append(hx(ins.getAddress())).append(',').append(hx(r.getToAddress()))
                                 .append(',').append(js(rt.getName())).append(']');
                        } else if (rt.isJump() && rt.isComputed()) {
                            anyJump = true;
                            if (jumps.length() > 0) jumps.append(',');
                            jumps.append('[').append(hx(ins.getAddress())).append(',').append(hx(r.getToAddress()))
                                 .append(',').append(js(rt.getName())).append(']');
                        }
                    }
                    FlowType ft = ins.getFlowType();
                    if (ft.isComputed() && (ft.isCall() ? !anyCall : (ft.isJump() && !anyJump))) {
                        if (unres.length() > 0) unres.append(',');
                        unres.append('[').append(hx(ins.getAddress())).append(',')
                             .append(js(ft.isCall() ? "call" : "jump")).append(']');
                    }
                }
                b.append(",\"calls\":[").append(calls).append(']');
                b.append(",\"jumps\":[").append(jumps).append(']');
                b.append(",\"unres\":[").append(unres).append("]}\n");
                w.write(b.toString());
            }
        }
        println("CAT7 noreturn_cleared=" + noretCleared + " callsites=" + callsitesFixed + " removed=" + removed + " overrides_cleared=" + overrides + " unthunked=" + unthunked + " created=" + created.size() + " failed=" + failed + " cleared_data_units=" + forced
                + " healed=" + healed + " conflicts_cleared=" + conflicts + " dumped=" + funcs.size() + " setbody_failed=" + setBodyFailed);
    }
}
