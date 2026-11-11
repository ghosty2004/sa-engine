# RW PowerPipe (Rx) pipelines, packets and clusters (RE-SPEC-RW-3 lead)

All functions are `__cdecl` unless stated. Engine = `RwEngineInstance` [0xc97b24]; memory functions through it:
`Engine+0x134` malloc(size, hint), `+0x138` free(p), `+0x13c` realloc(p, size, hint), `+0x144` freelist alloc(fl, hint),
`+0x148` freelist free(fl, p) (0x8057c7, 0x80585d, 0x8064ed, 0x805884, 0x8069f7). Errors: `_rwerror` 0x8088d0(code, arg) then
`RwErrorSet` 0x808820(&{1, result}); 0x80000013 = E_RW_NOMEM with the requested size as argument.

## Layouts (RW 3.6 SDK rwcore.h; offsets proven by the instructions cited)

- PIPEMODULE data `P = Engine + [0xc9bc60]`: +0x0 RwFreeList* of RxPipeline (0x805030), +0x4 default RxRenderStateVector
  (0x805074, filled by 0x80ea40), +0x30 = 0 and +0x34 = 0 at open (0x80509f, 0x80508c; meaning not used here), +0x38
  maxNodesPerPipeline (from [0x8e269c], 0x805065; read by 0x8069ae), +0x3c default atomic pipeline (pass 1, RpAtomic.md).
- RxPipeline (0x34 bytes; freelist element size 0x34 at 0x80501a; RxPipelineCreate clears 0xd dwords 0x8057d7):
  +0x0 locked, +0x4 numNodes, +0x8 nodes (RxPipelineNode[], stride 0x28 0x805841), +0xc packetNumClusterSlots,
  +0x10 embeddedPacketState (0 packetless / 1 unused / 2 in use / 3 pending; 0x80575b, 0x80577f), +0x14 embeddedPacket,
  +0x18 numInputRequirements, +0x1c inputRequirements, +0x20 superBlock, +0x24 superBlockSize, +0x28 entryPoint,
  +0x2c pluginId, +0x30 pluginData.
- RxPipelineNode (0x28): +0x0 nodeDef, +0x4 numOutputs, +0x8 outputs, +0xc slotClusterRefs, +0x10 slotsContinue,
  +0x14 privateData, +0x18 inputToClusterSlot, +0x1c topSortData, +0x20 initializationData, +0x24 initializationDataSize
  (pointer fix-up order in 0x8064d0: +8, +0xc, +0x10, +0x14, +0x18, +0x1c).
- RxNodeDefinition: +0x0 name, +0x4 nodeBody, +0x8 nodeInit, +0xc nodeTerm, +0x10 pipelineNodeInit, +0x14 pipelineNodeTerm,
  +0x18 pipelineNodeConfig, +0x1c configMsgHandler, +0x20 io.numClustersOfInterest, +0x24 io.clustersOfInterest
  (RxClusterRef stride 0xc), +0x28 io.inputRequirements, +0x2c io.numOutputs, +0x30 io.outputs, +0x34 privateDataSize,
  +0x38 editable, +0x3c InputPipesCnt (0x806a72).
- Execution context globals: [0xc9bc40] current pipeline, [0xc9bc48] exit code, [0xc9bc50] params.dataParam,
  [0xc9bc54] params.heap (0x80573d-0x805752). Global RxHeap* [0xc9a6b0]; module-opened flag [0xc9a6ac]
  (0x8050ac). Scratch stack window used while unlocking: top pointer [0xc9a6b4], bottom pointer [0xc9a6b8]
  (0x805890 / 0x8058f0).
- Module parameters in .data: [0x8e2698] = 4096 global heap superblock size, [0x8e269c] = 64 max nodes per pipeline,
  [0x8e26a0] = 64 pipeline freelist block size, [0x8e26a4] = 1 preallocated blocks (values from the image).

librw has no PowerPipe (it uses fixed ObjPipeline objects): every `- librw:` line below is "absent" unless stated.

## 0x804fe0 _rxPipelineOpen  (catalogue: specified/REA, port level 4)
- `RwBool (void)`, called only from cb_PIPEMODULE_RwEngine_Open 0x807c40. Returns EAX 1/0.
- If [0xc9a6ac] != 0 (already open, 0x804fe7): returns 0 (`xor eax,eax` at 0x80505c) without doing anything (note: returns
  FALSE, not TRUE, on a second open).
- `heap = RxHeapCreate 0x809f90([0x8e2698])` (0x804fef), stored to [0xc9a6b0]; NULL -> return 0.
- `P->freeList = RwFreeListCreateAndPreallocateSpace 0x801b70(0x34, [0x8e26a0], 4, [0x8e26a4], &[0xc9a688], 0x40409)` (args
  pushed at 0x80500c-0x80501a: entrySize 0x34, entriesPerBlock, alignment 4, numBlocksToPrealloc, static freelist storage
  0xc9a688, hint 0x40409). If the stored slot is NULL (0x80503e): `RxHeapDestroy 0x809f30([0xc9a6b0])`, [0xc9a6b0] = 0,
  return 0.
- Else: `P+0x38 = [0x8e269c]` (0x805065); `RxRenderStateVectorSetDefaultRenderStateVector 0x80ea40(P+4)` (0x805079);
  `P+0x34 = 0`, `P+0x30 = 0`; [0xc9a6ac] = 1; return 1.

## 0x805710 RxPipelineExecute  (catalogue: specified/REA, port level 4)
- `RxPipeline* (RxPipeline* pipeline [esp+4], void* data [esp+8], RwBool heapReset [esp+0xc])`.
- If heapReset != 0 and `[0xc9a6b0]->dirty (+0x18) != 0`: `_rxHeapReset 0x809ec0(heap)` (0x805718-0x805727; return ignored).
- Context: [0xc9bc48] = 1, [0xc9bc40] = pipeline, [0xc9bc50] = data, [0xc9bc54] = [0xc9a6b0] (0x80573d-0x805752);
  `pipeline+0x10 = 0` (packet state = packetless, 0x80575b).
- `node0 = pipeline->nodes (+8)`; calls `node0->nodeDef->nodeBody (+4)(node0, &[0xc9bc50])` (0x805766): always the FIRST node
  array entry, not `entryPoint` (+0x28). If it returns 0, [0xc9bc48] = 0 (0x805770).
- If `pipeline+0x10 > 1` (signed `jle`, 0x80577a): `pipeline+0x10 = 2`, `_rxPacketDestroy 0x80f070(pipeline+0x14)` (packet
  left in use/pending gets destroyed; the state is set to 2 before the call).
- Clears [0xc9bc40], [0xc9bc50], [0xc9bc54] to 0 ([0xc9bc48] is NOT cleared); returns `pipeline` if [0xc9bc48] != 0 else NULL
  (`neg/sbb/and`, 0x80579a-0x8057aa).

## 0x8057b0 RxPipelineCreate  (catalogue: specified/REA, port level 1)
- `RxPipeline* (void)`. `p = Engine+0x144(P->freeList (P+0), 0x30409)` (0x8057c7). If p: zero 13 dwords (0x34 bytes, `rep stosd`
  0x8057e0) and `p->locked (+0) = 0` again (0x8057e2); return p.
- On NULL: `_rwerror(0x80000013, 0x34)`, RwErrorSet, return NULL (0x8057eb-0x805810).

## 0x805890 sub_805890 (proposed: _rxPipelineScratchAllocTop)  (catalogue: unknown/REA, port level 1)
- `void* (RwUInt32 size [esp+4])`. `n = (size + 3) & ~3` (0x80589d-0x8058a5); `top = [0xc9a6b4] - n`, stored to [0xc9a6b4]
  (0x8058ac). If `top < [0xc9a6b8]` (unsigned, `jae` 0x8058b1 is the success branch): restore [0xc9a6b4] = top + n,
  `_rwerror(0x80000013, n)`, RwErrorSet, return NULL. Else return `top` (the new top, i.e. the allocated block start).
- Callers 0x80f460 (x2), 0x80f970, 0x80fc30 (all inside RxLockedPipeUnlock's helpers). Downward stack allocator over the
  scratch window set up by RxLockedPipeUnlock.

## 0x8058f0 sub_8058f0 (proposed: _rxPipelineScratchAllocBottom)  (catalogue: unknown/REA, port level 1)
- `void* (RwUInt32 size [esp+4])`. `n = (size + 3) & ~3`; `bot = [0xc9a6b8] + n`, stored (0x80590c). If `bot > [0xc9a6b4]`
  (unsigned, `jbe` 0x805911 = success): restore [0xc9a6b8] = bot - n, `_rwerror(0x80000013, n)`, RwErrorSet, return NULL.
  Else return the OLD bottom `bot - n` (0x805944). Upward allocator, same window; fails when it would cross the top pointer.

## 0x805950 sub_805950 (proposed: _rxPipelineCountClusterPasses)  (catalogue: unknown/REA, port level 0)
- `RwUInt32 (RxPipeline* pipe [esp+4])`, called once by 0x80f0d0. Locals: count = 0, `prev = 0` (EBP), n = pipe->numNodes.
- Outer loop (0x805968): `best = 0xffffffff`; for each node (stride 0x28 from pipe->nodes, 0x805996) and each RxClusterRef of
  its nodeDef (`nodeDef+0x20` count, `nodeDef+0x24` array, stride 0xc, 0x805990): `v = *(u32*)ref` (the first dword,
  `clusterDef` pointer used as an ordinal key); if `v > prev` and `v < best` (both unsigned, 0x805986, 0x80598a): best = v.
- If `best == 0xffffffff` (0x8059a0) return count; else `count++`, `prev = best` and repeat. If numNodes == 0 returns 0
  (0x80596f).
- Effect: returns the number of DISTINCT non-null cluster-definition pointers referenced by the pipeline's nodes (selection by
  increasing pointer value, so duplicates count once and NULL pointers are ignored). O(distinct * refs).

## 0x8064d0 sub_8064d0 (proposed: _rxPipelineResize, rw-names hint)  (catalogue: unknown/REA, port level 1)
- `RwBool (RxPipeline* pipe [esp+4], RwUInt32 newSize [esp+8])`. Callers RxLockedPipeUnlock (2 sites), RxPipelineLock.
- `nb = Engine+0x13c(pipe->superBlock (+0x20), newSize, 0x1040409)` (realloc, 0x8064ed). NULL -> `_rwerror(0x80000013, newSize)`,
  RwErrorSet, return 0 (pipe untouched).
- `delta = nb - oldSuperBlock` (0x80650c); `+0x20 = nb`, `+0x8 nodes = nb` (the node array always starts the superblock),
  `+0x24 = newSize`. If `+0x14` embeddedPacket != NULL it is moved by delta; same for `+0x1c` inputRequirements.
- For each of numNodes nodes (stride 0x28): each of +0x8, +0xc, +0x10, +0x14, +0x18, +0x1c is moved by delta when non-NULL
  (0x806531-0x80659e). +0x20 initializationData is not touched (separate allocation). Return 1.

## 0x8065e0 sub_8065e0 (proposed: _rxPipelineCopyInto, rw-names hint)  (catalogue: unknown/REA, port level 1)
- `RwBool (RxPipeline* dst [esp+4], RxPipeline* src [esp+8])`. Callers RxLockedPipeUnlock, RxPipelineLock (dst == src),
  RxPipelineClone. Uses `max = P+0x38` (maxNodesPerPipeline).
- If dst != src (0x8065ee): for i = src->numNodes-1 down to 0 (signed, skipped when numNodes == 0 via `js` 0x8065ff): copy the
  0x28-byte node `src->nodes[i]` to `dst->nodes[i]` (`rep movsd` 10 dwords, 0x80661a); clear dst node +0xc, +0x10, +0x14,
  +0x18 (0x806621-0x806636); if `+0x24 initializationDataSize > 0` (unsigned): `dst.+0x20 = Engine+0x134(size, 0x30409)`; NULL ->
  `_rwerror(0x80000013, size)`, RwErrorSet, return 0 (nodes already copied stay copied); else copy size bytes from the src
  node's +0x20 (dwords then bytes, 0x806680/0x80668b). Then `dst->numNodes = src->numNodes` (0x8066a6 / 0x8066ec).
- Layout (0x8066f3, also when dst == src): `outBase = dst->nodes + max*0x28`. For i = src->numNodes-1 down to 0:
  `dst->nodes[i].outputs (+8) = outBase + i*0x80` (0x80672a); if `src->nodes[i].outputs` (read AFTER that store, so for dst == src
  it is the new pointer) is non-NULL, copy 0x20 dwords from it to the new outputs (0x806749): always 32 outputs regardless of
  numOutputs.
- `ts = outBase + max*0x80` (0x806776); for i = 0..src->numNodes-1: `ts[0..2] = 0`, `dst->nodes[i].topSortData (+0x1c) = ts`,
  `ts += 0xc`. Return 1.
- Superblock layout implied: nodes[max] (0x28) | outputs[max][32] (0x80) | topSortData[max] (0xc) = 0xb4 * max bytes, which is
  exactly the size RxPipelineLock allocates.

## 0x8067b0 sub_8067b0 (proposed: _rxPipelineTopSort, rw-names hint)  (catalogue: unknown/REA, port level 0)
- `void (struct { RxPipeline* pipe; RwUInt32 next; }* ctx [esp+4], RwUInt32 idx [esp+8])`, recursive. Caller RxLockedPipeUnlock
  and itself (0x806972). Places node `idx` at sorted position `n = ctx->next` (+4) and visits its successors depth-first.
- If n != idx (0x8067c8): swap nodes n and idx so that the per-slot buffers stay at their slots:
  1. swap the 32 dwords of `nodes[n].outputs` and `nodes[idx].outputs` (loop count 0x20, 0x8067d6-0x806818), then swap the two
     outputs pointers (0x80681f, 0x80682c);
  2. swap the 3 dwords of the two topSortData blocks (0x806843-0x806880), then swap the two topSortData pointers;
  3. swap the whole 0x28-byte node structs through a stack temp (three `rep movsd` of 10 dwords, 0x8068a8-0x8068c8), which swaps
     the pointers back: net effect, node contents moved, buffers stay attached to slot positions.
  4. renumber: for every node j and each of its numOutputs outputs (0x8068eb loop): value == n -> idx; else value == idx -> n.
- 0x806925: `ctx->next = n + 1`; for each output k of `nodes[n]` (numOutputs at +4, skipped when 0): t = outputs[k]; if t != -1:
  `nodes[t].topSortData->numInsVisited (+4) += 1`; if `numIns (+0) == numInsVisited` recurse `(ctx, t)` (0x806970).
- Recursion depth is bounded by the node count (max 64 by default).

## 0x806990 RxPipelineLock  (catalogue: specified/REA, port level 2)
- `RxPipeline* (RxPipeline* pipe [esp+4])`. If `pipe->locked (+0) != 0` returns pipe unchanged (0x80699d).
- `size = P+0x38 * 0xb4` (`lea *5`, `lea *9`, `shl 2`, 0x8069ae-0x8069bb).
- If `pipe->nodes (+8) == NULL` (new pipeline): `sb = Engine+0x134(size, 0x1030409)` (0x8069f7), stored to +0x20; NULL ->
  `_rwerror(0x80000013, size)`, RwErrorSet, return NULL. Else `+0x24 = size`, `+8 = sb` (0x806a34) and continue at 0x806a3a
  (no copy/layout call).
- Else: if `size > +0x24` (unsigned) call 0x8064d0(pipe, size); 0 -> return NULL. Then 0x8065e0(pipe, pipe) (re-layout outputs and
  topSortData in place); 0 -> return NULL.
- 0x806a3a: `locked = 1`. If nodes != NULL, for each node i < numNodes: if `nodeDef->pipelineNodeTerm (+0x14)` call it(node)
  (0x806a67); `nodeDef->InputPipesCnt (+0x3c) -= 1`; if it is now 0 and `nodeDef->nodeTerm (+0xc)` non-NULL call it(nodeDef)
  (0x806a8e); `node->slotClusterRefs (+0xc) = 0`. Return pipe.
- Note: unlike the SDK docs, an already-locked pipeline is returned (non-NULL) silently.

## 0x806b30 RxPipelineFindNodeByName  (catalogue: specified/REA, port level 0)
- `RxPipelineNode* (RxPipeline* pipe [esp+4], const RwChar* name [esp+8], RxPipelineNode* start [esp+0xc], RwInt32* nodeIndex [esp+0x10])`.
- Not found path (0x806b93): `if (nodeIndex) *nodeIndex = -1`; return NULL. Taken when pipe == NULL, name == NULL or
  numNodes == 0 (0x806b3a-0x806b49).
- `cur = pipe->nodes`, `left = numNodes`. If start != NULL: advance `cur += 0x28; left--` until `cur == start` or `left <= 0`
  (signed, 0x806b5a-0x806b64), then step once more past it (0x806b66); the search begins at the node AFTER start.
- While `left > 0` (signed): if `cur->nodeDef != NULL` and `Engine+0x114 (stringFuncs.vecStrcmp)(cur->nodeDef->name (+0), name) == 0`
  (0x806b7e) -> found; else `cur += 0x28; left--`.
- Found (0x806ba8): `if (nodeIndex) *nodeIndex = numNodes - left`; return cur. Name compare is case-sensitive (strcmp slot).

## 0x806be0 RxLockedPipeAddFragment  (catalogue: specified/REA, port level 2)
- `RxPipeline* (RxPipeline* pipe [esp+4], RwUInt32* firstIndex [esp+8], RxNodeDefinition* def0 [esp+0xc], ...)`, varargs cdecl,
  NULL-terminated list of node definitions.
- pipe == NULL: `_rwerror(0x80000016)` (E_RW_NULLP, no second argument), RwErrorSet, return NULL (0x806d85).
- `pipe->locked == 0`: `_rwerror(0x34)` (plain 0x34, pushed at 0x806db2), RwErrorSet, return NULL.
- def0 == NULL: return NULL without error (0x806c05). n = number of non-NULL defs (0x806c0f loop).
- `start = numNodes`; if `start + n > P+0x38` (unsigned, 0x806c3e): `_rwerror(0x2a)`, RwErrorSet, return NULL.
- For i = 0..n-1 (0x806c65): `node = pipe->nodes + (start+i)*0x28` (nodes re-read every pass); `0x806de0(pipe, node, def_i)`;
  0 -> stop (cleanup). For i > 0, connect the previous node's output 0 to this node (inlined RxLockedPipeAddPath logic):
  `cur = (node && node->nodeDef) ? node : NULL`; `out = (prev->nodeDef && prev->numOutputs > 0) ? prev->outputs : NULL`; the link
  is made only if `pipe->locked && out && *out == -1 && cur && cur->nodeDef`, and the index `k = (cur - nodes) / 0x28` (signed
  division by 40 via 0x66666667, 0x806cd0) satisfies `nodes + k*0x28 == cur`, `k < numNodes`, `k != -1`; then `*out = k`
  (0x806cfd). Any failed check: `0x8050c0(node, pipe)` destroys the node just added (0x806d2a) and goes to cleanup.
  `added++` after each success.
- Cleanup 0x806d32: if added == n: `if (firstIndex) *firstIndex = start`; return pipe. Else if added != 0: for k = added down to 1
  call `0x8050c0(pipe->nodes + (start+k-1)*0x28, pipe)` (reverse order, 0x806d66-0x806d79); return NULL.

## 0x806de0 sub_806de0 (proposed: _rxPipelineNodeInitFromDefinition, rw-names hint)  (catalogue: unknown/REA, port level 1)
- `RwBool (RxPipeline* pipe [esp+4], RxPipelineNode* node [esp+8], RxNodeDefinition* def [esp+0xc])`. Callers AddFragment,
  RxLockedPipeReplaceNode. Assumes `node == &pipe->nodes[pipe->numNodes]`.
- Zeroes the 0x28-byte node (0x806dfb). ok = 1. `numOut = def->io.numOutputs (+0x2c)`.
- If numOut > 0x20 (unsigned): `_rwerror(0x29)`, RwErrorSet, ok = 0 (continues). If `def->io.numClustersOfInterest (+0x20) > 0x20`:
  `_rwerror(0x28)`, RwErrorSet, ok = 0.
- If `numOut >= P+0x38` (unsigned; it compares the OUTPUT count with maxNodesPerPipeline, 0x806e63): `_rwerror(0x2a)`, return 0.
- If ok == 0 return 0 (node left zeroed).
- `node->outputs (+8) = nodes + max*0x28 + numNodes*0x80`; `node->numOutputs (+4) = numOut`; all numOut entries = -1 (0x806eb6).
- `ts = nodes + max*168 + numNodes*12` (0x806ed6-0x806eeb: the topSortData slot of index numNodes); ts[0..2] = 0;
  `node+0x1c = ts`, `+0x20 = 0`, `+0x24 = 0`, `node->nodeDef = def` (0x806f03); `pipe->numNodes += 1`. Return 1.
- Note: InputPipesCnt of def is not touched here (done at unlock time).

## 0x8050c0 sub_8050c0 (proposed: _rxPipelineNodeDestroy; rw-names hint _rxPipelineNodeDestroyRecurse)  (catalogue: unknown/REA, port level 0)
- `RxPipelineNode* (RxPipelineNode* node [esp+4], RxPipeline* pipe [esp+8])`; returns node. Callers _rxPipelineDestroy,
  RxLockedPipeAddFragment, RxLockedPipeReplaceNode, RxLockedPipeDeleteNode. No recursion.
- Pipeline NOT locked (`pipe->locked == 0`, 0x8050d2): `def = node->nodeDef`; if `def->pipelineNodeTerm (+0x14)` call it(node);
  `def->InputPipesCnt (+0x3c) -= 1`; if now 0: call `def->nodeTerm (+0xc)(def)` if non-NULL, and if `def->editable (+0x38) != 0`
  free(def) via Engine+0x138 and `node->nodeDef = 0` (0x805117). Then if `node+0x20` initializationData: free it, +0x20 = +0x24 = 0.
  Zero the whole node (10 dwords, 0x80513f); `pipe->numNodes -= 1` (no compaction: only valid for the last node). Return node.
- Pipeline locked (0x80514f): free initializationData as above; `def = node->nodeDef` (dereferenced unconditionally); if
  `def->InputPipesCnt == 0 && def->editable` free(def), nodeDef = 0. `idx = (node - pipe->nodes) / 0x28` (signed /40 via 0x66666667).
  If `idx < numNodes - 1` (unsigned, 0x8051a9), close the gap (max = P+0x38, outBase = nodes + max*0x28, tsBase = outBase + max*0x80):
  1. for j = idx .. numNodes-2: copy the 32-dword output buffer of slot j+1 onto slot j (0x8051d3);
  2. for j = idx .. numNodes-2: topSort slot j = slot j+1 (3 dwords, 0x80521a);
  3. for j = idx .. numNodes-2: `nodes[j] = nodes[j+1]` (10 dwords), then `nodes[j].outputs -= 0x80`, `nodes[j].topSortData -= 0xc`
     (0x805267, 0x805277);
  4. renumber, for each of the first numNodes-1 nodes and each output k < numOutputs (0x80529f): `v < idx` (unsigned) keep;
     `v == idx` -> 0xffffffff; else `v -= 1`. ORIGINAL BUG: an unconnected output (0xffffffff) is "greater" and becomes 0xfffffffe.
  `pipe->numNodes -= 1` (0x8052d9); return node. When idx is the last node nothing is moved or renumbered (outputs still
  pointing at it keep the stale index).

## 0x805f40 RxLockedPipeUnlock  (catalogue: specified/REA, port level 4)
- `RxPipeline* (RxPipeline* pipe [esp+4])`; returns pipe (now unlocked) or NULL. Context struct on stack `ctx = {pipe, next = 0}`
  at [esp+0x10] for 0x8067b0.
- Checks: pipe NULL -> `_rwerror(0x80000016)`, NULL (0x80646d). Not locked -> `_rwerror(0x34)`, NULL (0x80649a). numNodes == 0 ->
  `locked = 0`, return pipe (0x8063b2). `entryPoint (+0x28) >= numNodes` or `nodes[entryPoint].nodeDef == NULL` -> `_rwerror(0x24)`,
  NULL (0x806443).
- K = number of distinct non-NULL clusterDef pointers over all nodes' clustersOfInterest (same algorithm as 0x805950, inlined
  0x805f86-0x805fd4).
- Superblock size (all unsigned, n = numNodes, m = P+0x38, 0x805fd6-0x806072):
  `A = 40n + (n > 0 ? 4 * n * nodes[0].numOutputs : 0)` (uses node 0's numOutputs for every node, 0x806002-0x806008);
  `B = 12m + n*(52K + 20) + A`; `C = 4*((n+5)*K + (K+1)*n) + sum over nodes(def->privateDataSize (+0x34) + 4*def->io.numClustersOfInterest (+0x20)) + 28K`;
  `need = C + B + 0x14`; `need = max(need, 180m)` (`jbe` at 0x806078 keeps 180m when need <= 180m).
  If `need > superBlockSize (+0x24)`: `0x8064d0(pipe, need)`; 0 -> return NULL (no extra error).
- Scratch window: [0xc9a6b8] = 0, [0xc9a6b4] = superBlock + need (0x80609e, 0x8060a4; both re-set below before use).
- Topological sort: for every node with nodeDef: `ts->numInsVisited = 0`, `ts->numIns = 0`; then for every node with nodeDef and
  numOutputs != 0, each output o != -1: `nodes[o].topSortData->numIns += 1` (0x806105).
  If `nodes[entryPoint].ts->numIns != 0` -> `_rwerror(0x24)`, NULL. Any other node i (i != entryPoint) with `numIns == 0` ->
  `_rwerror(0x22)`, NULL (0x80617e). `0x8067b0(&ctx, entryPoint)` (0x806154). Any node with `numIns != numInsVisited` ->
  `_rwerror(0x1c)`, NULL (0x8061a8). Then `entryPoint = 0` (0x8061d2).
- Compaction (0x8061d5-0x8062c3), outBase = nodes + m*0x28, tsBase = outBase + m*0x80:
  1. topSortData moved to the top of the needed area: for i = n-1 down to 0, copy ts slot i (3 dwords) to
     `superBlock + need - 12*(n-i)` and point `nodes[i].topSortData` there (store 0x80623e).
  2. outputs packed right after the node array: dst = nodes + n*0x28; for i = 0..n-1: c = numOutputs; c == 0 -> `outputs = NULL`;
     else copy c dwords from slot buffer `outBase + i*0x80` to dst, `outputs = dst`, dst += 4c (0x806282).
  3. Scratch window for the next phase: top [0xc9a6b4] = tsBase - 12 (ESI after step 1, 0x8062ce), bottom [0xc9a6b8] =
     outBase + n*0x80 + 4*sum(numOutputs) (0x8062d7).
- `r = 0x80f0d0(pipe)` (0x8062dd): r != 0 -> return NULL (no error, no relayout).
- `0x8064d0(pipe, [0xc9a6b8] - superBlock)` (shrinks to the bottom pointer); 0 -> relayout (0x80642f) and NULL.
- All `topSortData = 0` (0x80631a).
- Init, i = n-1 down to 0 (0x80633a): `def->InputPipesCnt++`; if it was 0 and `def->nodeInit (+8)`: call it(def); 0 -> undo.
  If `def->pipelineNodeInit (+0x10)`: call it(node); 0 -> `InputPipesCnt--`, if 0 call nodeTerm (+0xc)(def), then undo.
  Undo (0x8063d8): for k = i+1 .. n-1: `pipelineNodeTerm (+0x14)(node_k)` if set; `InputPipesCnt--`; if 0 and nodeTerm, nodeTerm(def);
  then `0x8065e0(pipe, pipe)`, return NULL. (Node i itself is not term'd by the loop.)
- Config, i = n-1 down to 0 (0x80638f): if `def->pipelineNodeConfig (+0x18)`: `cfg(node, pipe)`; 0 -> undo ALL nodes 0..n-1 as above
  (0x8063e6), relayout, NULL.
- Success: `locked = 0`, return pipe (0x8063b4).

## Unlock-time dependency structures (built in the scratch window, proven by 0x80f460, 0x80f4b0, 0x80f970)

- rxReq (0x14 bytes, one per node, `topSortData->req` at ts+8, 0x80f16f): +0x0 numEntries, +0x4 capacity, +0x8 slot bit mask (filled
  by 0x80fba0), +0xc entries (capacity x 0x24), +0x10 owning RxPipelineNode*.
- req entry (0x24): +0x0 RxClusterDefinition*, +0x4 requirement (0 DONTWANT / 1 REQUIRED / 2 OPTIONAL), +0x8 reference count,
  +0xc group*, +0x10 next entry in group, +0x14 owning rxReq*, +0x18 bit mask of node outputs the cluster continues through,
  +0x1c slot index (init -1), +0x20 RxPipelineNode* of the requiring node.
- group (0x10, 0x80f9b2): +0x0 entry list head, +0x4 next merged group, +0x8 next group in the global list, +0xc parent group.
- RxClusterValid of an output (`def->io.outputs + k*0xc`: +0x4 outputClusters[], +0x8 allOtherClusters): 0 NOCHANGE, 1 VALID,
  2 INVALID. Lookup rule used everywhere: `ci = index of clusterDef in def->io.clustersOfInterest`; `ci == -1 ? allOtherClusters :
  outputClusters[ci]`.

## 0x80f0d0 sub_80f0d0 (proposed: _rxPipelineUnlockResolve; rw-names hint _rxPipelineUnlockBuildClusters)  (catalogue: unknown/REA, port level 3)
- `RwUInt32 (RxPipeline* pipe [esp+4])`: returns 0 on success or the error code it reported (0x1d, 0x1e, 0x1f, 0x20). Caller
  RxLockedPipeUnlock (0x8062dd) treats non-zero as failure.
- Backward pass over the (already topologically sorted) nodes, i = numNodes-1 down to 0 (counter at [esp+0x2c], node pointer
  steps -0x28, 0x80f35a):
  1. clustersOfInterest validation: for each i: `cd_i == NULL` -> `_rwerror(0x1f, def->name, i)`, return 0x1f (0x80f420); for j > i
     with `cd_j == cd_i` -> `_rwerror(0x1e, def->name, cd_i)`, return 0x1e (0x80f3eb).
  2. `req = 0x80f460(node, 0x805950(pipe))` (capacity = distinct cluster count, recomputed per node, 0x80f15b); `ts->req = req`;
     NULL -> `_rwerror(0x20)`, return 0x20 (0x80f3be).
  3. For each output k with target t != -1, and each entry e of `nodes[t].ts->req` (in index order): v = validity of e.cd on output k.
     v == 0: `0x80f4b0(req, e.cd, e.requirement, ts->numIns, e.node (+0x20))` (inherit the requirement, 0x80f21b).
     else if `e.requirement == 1`: v == 1 is fine; any other v -> `_rwerror(0x1d, e.cd->name, e.node->nodeDef->name, def->name, k,
     outputSpec->name)` (pushed 0x80f276-0x80f28a), return 0x1d.
     else if v == 2: `0x80f570(pipe, &nodes[t], e.cd)` (drop the optional requirement downstream).
  4. For each clusterOfInterest i with `def->io.inputRequirements[i] != 0`: `0x80f4b0(req, cd_i, inputRequirements[i], ts->numIns, node)`.
  (0x80f4b0 never returns NULL, so the 0x20 error checks after it are dead.)
- Then `r = 0x80f690(pipe)`; `r = 0x80f970(pipe, &groups)` (groups list head at [esp+0x28], initialised 0 at 0x80f0e8);
  `r = 0x80fba0(pipe, &groups)`; `r = 0x80fc30(pipe, groups)`; the first non-zero r is returned (0x80f380-0x80f3a6).

## 0x80f460 sub_80f460 (proposed: _rxReqCreate; rw-names hint _rxPipelineUnlockAllocNodeSet)  (catalogue: unknown/REA, port level 2)
- `rxReq* (RxPipelineNode* node [esp+4], RwUInt32 capacity [esp+8])`. `r = 0x805890(0x14)`; NULL -> NULL. `r->entries = 0x805890(capacity*0x24)`
  (0x80f47d); NULL -> NULL (the 0x14 bytes stay allocated). `+4 = capacity`, `+0 = 0`, `+0x10 = node`, `+8 = 0`. Returns r.

## 0x80f4b0 sub_80f4b0 (proposed: _rxReqAddRequirement; rw-names hint _rxPipelineUnlockMarkPaths)  (catalogue: unknown/REA, port level 0)
- `entry* (rxReq* req, RxClusterDefinition* cd, RwUInt32 requirement, RwUInt32 refCount, RxPipelineNode* node)` (args [esp+4..+0x14]).
- Linear search of the entries for `cd` (stride 0x24, 0x80f4c9). Found: if requirement == 1 then `e+4 = 1` and `e+0x20 = node`
  (0x80f527-0x80f530); otherwise unchanged; return e.
- Not found: e = entries[num]; `num += 1`; e = {cd, requirement, refCount, +0xc 0, +0x10 0, +0x14 req, +0x18 0, +0x1c -1, +0x20 node};
  return e. NO capacity check (capacity = distinct cluster count, so it cannot overflow with consistent input).

## 0x80f540 sub_80f540 (proposed: _rxIoSpecFindCluster; rw-names hint _rxPipelineUnlockFindCluster)  (catalogue: unknown/REA, port level 0)
- `RwInt32 (RxIoSpec* io [esp+4], RxClusterDefinition* cd [esp+8])`: index i < io->numClustersOfInterest (+0) with
  `io->clustersOfInterest[i].clusterDef == cd` (array at +4, stride 0xc), else -1 (0x80f560).

## 0x80f570 sub_80f570 (proposed: _rxReqReleaseRequirement; rw-names hint _rxPipelineUnlockPropagate)  (catalogue: unknown/REA, port level 0)
- `void (RxPipeline* pipe, RxPipelineNode* node, RxClusterDefinition* cd)`, recursive. Find the entry for cd in `node->ts->req`;
  absent (or empty req) -> return. `e+8 -= 1`; if still non-zero return. Else remove it: copy the LAST entry over it (9 dwords) unless it
  is the last, `num -= 1` (0x80f5da-0x80f5e7).
- Then for each output k with target t != -1: v = validity of cd on output k (lookup inlined 0x80f61a); if v == 0 recurse
  `(pipe, &nodes[t], cd)` (0x80f658).

## 0x80f690 sub_80f690 (proposed: _rxPipelineUnlockForwardPass; rw-names hint _rxPipelineUnlockResolveClusters)  (catalogue: unknown/REA, port level 1)
- `RwUInt32 (RxPipeline* pipe)`; forward over nodes 0..numNodes-1 (0x80f8c7). Returns 0, or 0x20 (dead error paths).
- For each clusterOfInterest i with `forcePresent (+4) != 0`: ensure an entry for cd_i in the node's req; when missing append
  `{cd, requirement 0, refCount 1, 0, 0, req, 0, -1, node}` (inlined add, 0x80f6f4-0x80f725); an existing entry is left unchanged.
- For each output k with target t != -1 and each entry e of `nodes[t].ts->req`: v = validity of e.cd on output k:
  v == 2 -> nothing; v == 0 -> if the node's own req has an entry for e.cd set bit k in its +0x18 (0x80f866-0x80f871), else nothing;
  v == 1 (or other) -> find or append (requirement 0, refCount 1, node) the own entry, then set bit k in its +0x18.

## 0x80f970 sub_80f970 (proposed: _rxPipelineUnlockGroupClusters; rw-names hint _rxPipelineUnlockAssignSlots)  (catalogue: unknown/REA, port level 2)
- `RwUInt32 (RxPipeline* pipe, group** list)`; forward over nodes. Returns 0 or 0x20 when a group allocation fails (0x80fb73).
- For each entry e of the node's req with `e+0xc == NULL`: `g = 0x805890(0x10)`; g = {head 0, next 0, parent 0, listNext = *list};
  `*list = g` (prepend, 0x80f9d1); `e+0xc = g`; `e+0x10 = g->head`; `g->head = e`.
- For each output k with target t != -1 and each entry te of the target's req with `te+4 != 0`: find own entry oe with the same cd and bit k
  set in `oe+0x18`; if none skip. If `te+0xc == NULL`: te joins oe's group (`te+0x10 = group->head`, `group->head = te`, `te+0xc = group`).
  Else: ra = root of te's group, rb = root of oe's group (follow +0xc); if different: append rb at the end of ra's +4 chain, `rb+0xc = ra`,
  unlink rb from *list (walk +8 links, 0x80fafc-0x80fb15).

## 0x80fba0 sub_80fba0 (proposed: _rxPipelineUnlockAssignSlots; rw-names hint _rxPipelineUnlockCountRequirements)  (catalogue: unknown/REA, port level 0)
- `RwUInt32 (RxPipeline* pipe, group** list)`, returns 0. slots = 0. For each group g in *list (via +8): mask = OR of `entry->req->+8`
  over all entries of all groups in g's +4 chain; slot = number of trailing 1 bits of mask (0x80fbce-0x80fbd9); `slots = max(slots,
  slot+1)`; for every such entry: `+0x1c = slot`, `req+8 |= 1 << slot` (0x80fbf4, 0x80fbf7). Finally `pipe+0xc = slots` (also 0 when the
  list is empty, 0x80fc23).

## 0x80fc30 sub_80fc30 (proposed: _rxPipelineUnlockBuild; rw-names hint _rxPipelineUnlockFinalise)  (catalogue: unknown/REA, port level 2)
- `RwUInt32 (RxPipeline* pipe, group* list)`, returns 0. All allocations from the scratch bottom (0x8058f0); results are not
  NULL-checked.
  1. R = number of node-0 req entries with requirement 1 or 2 (0x80fc61-0x80fc6e). G = number of groups in the list whose head entry's cd
     does not occur in an EARLIER group of the list (0x80fc8b-0x80fca8).
  2. Zero the free scratch window: `top = 0x805890(0)`, `bot = 0x8058f0(0)`, memset(bot, 0, top - bot) (0x80fcd1).
  3. `pkt = alloc(28*slots + 0x14)`; `pipe+0x14 = pkt`; `pkt+2 (u16) = (u16)slots`; `pkt+4 = pipe`; `pipe+0x10 = 0` (0x80fcf3-0x80fd0b).
  4. RxPipelineCluster array: for each first-occurrence group (same test as G): `pc = alloc(8)` = {cd, cd->defaultAttributes (+8)}; the
     first pc is the array base (inlined 0x80ffb0; consecutive bump allocations make it contiguous).
  5. For each node: if slots: `+0xc slotClusterRefs = alloc(4*slots)`; `+0x10 slotsContinue = alloc(4*slots + 4)`; if numCOI:
     `+0x18 inputToClusterSlot = alloc(4*numCOI)`; if `def->privateDataSize`: `+0x14 privateData = alloc((size+3) & ~3)`.
     mask = -1; for each req entry e: `slotClusterRefs[e.slot] = &pc[j]` where pc[j].clusterRef == e.cd (NULL if none), 
     `slotsContinue[e.slot + 1] = e+0x18`, `mask &= e+0x18`; then `slotsContinue[0] = mask` (0x80fe77).
     For each COI k: `inputToClusterSlot[k] = -1`, then the first slot j with `slotClusterRefs[j] && slotClusterRefs[j]->clusterRef == cd_k`
     (0x80fe9a-0x80fec1).
  6. If R: `pipe+0x1c = alloc(12*R)`; copy {cd, requirement, slot} of node 0's REQUIRED/OPTIONAL entries; `pipe+0x18 = R`;
     `0x8103d0(pipe+0x1c, R, 0xc, 0, 0, 0xffffffff)` sorts them by cd pointer (0x80ff92).

## 0x80ffb0 sub_80ffb0 (proposed: _rxPipelineClusterCreate)  (catalogue: unknown/REA, port level 2)
- `RxPipelineCluster* (RxClusterDefinition* cd [esp+4], ? [esp+8] unused, RxPipelineCluster** first [esp+0xc])`. `pc = 0x8058f0(8)`
  (not NULL-checked); if `*first == NULL` then `*first = pc`; `pc->clusterRef = cd`, `pc->creationAttributes = cd+8`. Returns pc. Its
  only use is as the inlined callback in 0x80fc30 (address loaded at 0x80fd33 only for a non-NULL test).

## 0x8103d0 sub_8103d0 (proposed: _rwRadixExchangeSort; rw-names hint _rxPipelineUnlockSortRequirements)  (catalogue: unknown/REA, port level 0)
- `void (void* base, RwUInt32 count, RwUInt32 stride, RwUInt32 keyOffset, RwUInt32 keyMin, RwUInt32 keyMax)`; key = u32 at
  `elem + keyOffset`. Ascending, unsigned, NOT stable. Element swaps copy `stride/4` dwords (nothing is swapped when stride < 4).
- Returns at once if base == NULL, `keyOffset + 4 > stride`, or `keyMin >= keyMax` (0x8103e3-0x810412).
- If count > 5: binary radix-exchange sort. topBit = `1 << floor(log2(keyMax))` (keyMax == 0 would give shift count -1, i.e. 31).
  Explicit stack of {lo, hi, bit} at [esp+0x24] (stride 0xc), seeded with {base, base + (count-1)*stride, topBit}. Partition: lo pointer
  advances while `key & bit == 0`, hi pointer retreats while `key & bit != 0`, swap, step both; stop when lo > hi. Then `bit >>= 1`; if bit
  != 0: the upper part [hi+stride, oldHi] is pushed when `oldHi >= hi + 6*stride` (0x8104df-0x8104fa), and the lower part
  [oldLo, lo-stride] is processed next when `lo - stride >= oldLo + 5*stride` (0x81050e); otherwise pop. Smaller parts are left to
  the insertion sort.
- If count > 1: sentinel: among elements 0..m, m = min(4, count-1), find the minimum key scanning m down to 0 with strict `<` (ties keep the
  higher index), swap it with element 0 (0x81054b-0x8105a6). Then insertion sort i = 1..count-1 by adjacent swaps while
  `key[j-1] > key_i` (unsigned, 0x8105ff).

## 0x804f60 _rxPipelineClose  (catalogue: specified/REA, port level 1)
- `RwBool (void)`. If [0xc9a6ac] != 0: `RwFreeListDestroy 0x801b80(P+0)`, `P+0 = 0`, `RxHeapDestroy 0x809f30([0xc9a6b0])`, [0xc9a6b0] = 0,
  [0xc9a6ac] = 0. Always returns 1.

## 0x807c40 cb_PIPEMODULE_RwEngine_Open  (catalogue: specified/REA, port level 5)
- RwEngine plugin open callback `void* (void* instance [esp+4], RwInt32 offset [esp+8], RwInt32 size)`: `[0xc9bc60] = offset` (PIPEMODULE
  offset global, 0x807c44); `r = _rxPipelineOpen 0x804fe0()`; returns `r ? instance : NULL` (`neg/sbb/and`, 0x807c52-0x807c56).

## 0x807c60 cb_PIPEMODULE_RwEngine_Close  (catalogue: specified/REA, port level 2)
- `void* (void* instance, RwInt32 offset, RwInt32 size)`: calls `_rxPipelineClose 0x804f60()` and returns instance (result ignored).

## 0x805300 RxPacketCreate  (catalogue: specified/REA, port level 0)
- `RxPacket* (RxPipelineNode* node [esp+4])`. `pipe = [0xc9bc40]` (executing pipeline): `pipe+0x10 = 2` (in use); `pkt = pipe+0x14`;
  `pkt->inputToClusterSlot (+8) = node+0x18`; `pkt->slotsContinue (+0xc) = node+0x10`; `pkt->slotClusterRefs (+0x10) = node+0xc`.
  Returns EAX = pkt (left in EAX from 0x805312). Packet clusters are not cleared here.

## 0x805330 RxClusterSetStride  (catalogue: specified/REA, port level 0)
- `RxCluster* (RxCluster* c, RwInt32 stride)`: `*(u16*)(c+2) = (u16)stride`; returns c (EAX = c from the first load).

RxCluster layout (0x1c): +0x0 flags (u16; bit 0 = valid, bit 1 = external data), +0x2 stride (u16), +0x4 data, +0x8 currentData,
+0xc numAlloced, +0x10 numUsed, +0x14 clusterRef, +0x18 attributes. Heap = [0xc9a6b0].

## 0x805340 RxClusterSetExternalData  (catalogue: specified/REA, port level 3)
- `RxCluster* (RxCluster* c, void* data, RwInt32 stride, RwInt32 numElements)`. If `c->data && !(flags & 2) && c->data != data`:
  `RxHeapFree(heap, c->data)`, `c->data = 0` (0x80535a-0x805369). Then `flags |= 3` (byte or, 0x805379), `data = currentData = data`,
  `numAlloced = numUsed = numElements`, `stride = (u16)stride`. Returns c.

## 0x8053a0 RxClusterSetData  (catalogue: specified/REA, port level 3)
- Same as 0x805340 but `flags |= 1` (0x8053d9): the data is owned (heap) data. Same free rule for the previous data.

## 0x805400 RxClusterInitializeData  (catalogue: specified/REA, port level 4)
- `RxCluster* (RxCluster* c, RwUInt32 numElements, RwUInt16 stride)`. `bytes = (stride & 0xffff) * numElements` (imul, 0x80541b). If
  `c->data && !(flags & 2)`: RxHeapFree it (no pointer reset needed, overwritten). `p = RxHeapAlloc(heap, bytes)` (NOT NULL-checked);
  `flags |= 1`; `data = currentData = p`; `numAlloced = numElements`; `numUsed = 0`; `stride = stride`. Returns c.

## 0x805470 RxClusterResizeData  (catalogue: specified/REA, port level 5)
- `RxCluster* (RxCluster* c, RwUInt32 numElements)`. `p = RxHeapRealloc(heap, c->data, stride * numElements, 1)` (a fourth argument 1 is
  pushed; RxHeapRealloc ignores it); `flags |= 1`; `data = currentData = p` (cursor reset to the start, result not NULL-checked);
  `numAlloced = numElements`; if `numUsed > numElements` (unsigned) `numUsed = numElements`. Returns c. External data is realloc'ed too
  (no flag test).

## 0x8054c0 RxClusterDestroyData  (catalogue: specified/REA, port level 3)
- `RxCluster* (RxCluster* c)`: if `!(flags & 2)` `RxHeapFree(heap, c->data)` (even when data is NULL); then `data = 0`, `numAlloced = 0`,
  `numUsed = 0`, `flags = 0` (u16). Returns EAX = 0 (the `xor eax,eax` value), not c.

## 0x8054f0 RxClusterLockWrite  (catalogue: specified/REA, port level 4)
- `RxCluster* (RxPacket* pkt [esp+4], RwUInt32 clusterIndex [esp+8], RxPipelineNode* node)` (node unused). RxPacket: +0x8
  inputToClusterSlot, +0xc slotsContinue, +0x10 slotClusterRefs, +0x14 clusters[] (stride 0x1c).
- `slot = pkt->inputToClusterSlot[clusterIndex]`; -1 -> return NULL (0x8055bb). `c = pkt + 0x14 + slot*0x1c`.
- If `c->clusterRef (+0x14) == NULL`: `c->clusterRef = pkt->slotClusterRefs[slot]`, `c->attributes (+0x18) = clusterRef->creationAttributes (+4)`.
- `flags |= 8` (byte or, 0x80552c). If `(flags & 6) == 2` (external, not marked modifiable): `flags &= ~2`; save stride, numAlloced,
  numUsed and old data; `data = 0`; `p = RxHeapAlloc(heap, stride*numAlloced)` (not NULL-checked); `flags |= 1`; restore stride/numAlloced/
  numUsed; `data = currentData = p`; copy `stride*numUsed` bytes from the old data (0x8055a5-0x8055ac).
- Always `currentData = data` (0x8055b4); return c.

## 0x8055d0 RxPipelineNodeSendConfigMsg  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RxPipelineNode* node, RwUInt32 msg, RwUInt32 intParam, void* ptrParam)`: node NULL -> 0xffffffff; `h = node->nodeDef->configMsgHandler (+0x1c)`;
  NULL -> 0; else tail-jump to h with the caller's arguments unchanged (0x8055e5).

## 0x8055f0 RxPipelineNodeForAllConnectedOutputs  (catalogue: specified/REA, port level 0)
- `RxPipelineNode* (RxPipelineNode* node, RxPipeline* pipe, RxPipelineNodeOutputCallBack cb, void* data)`. Returns NULL when node or pipe is NULL,
  node is outside `[pipe->nodes, pipe->nodes + numNodes*0x28)` (0x805605-0x805614), or cb is NULL.
- For k = 0 .. node->numOutputs-1 (re-read each pass): `t = outputs[k]`; if `t < pipe->numNodes` (unsigned, so -1 is skipped) call
  `cb(node, &pipe->nodes[t], data)`; 0 -> stop. Returns node.

## 0x805680 RxPipelineNodeGetPipelineCluster  (catalogue: specified/REA, port level 0)
- `RxPipelineCluster* (RxPipelineNode* node, RwUInt32 clusterIndex)`: NULL if node NULL, `clusterIndex >= nodeDef->io.numClustersOfInterest (+0x20)`
  or `node->inputToClusterSlot[clusterIndex] == -1`; else `node->slotClusterRefs[slot]`.

## 0x8056b0 RxPipelineClusterGetCreationAttributes  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RxPipelineCluster* pc)`: pc ? `pc->creationAttributes (+4)` : 0xffffffff.

## 0x8056c0 RxPipelineClusterSetCreationAttributes  (catalogue: specified/REA, port level 0)
- `RxPipelineCluster* (RxPipelineCluster* pc, RwUInt32 attr)`: if pc: `pc+4 = attr`, return pc; else return NULL.

## 0x8056e0 RxClusterGetAttributes  (catalogue: specified/REA, port level 0)
- `RwUInt32 (RxCluster* c)`: c ? `c->attributes (+0x18)` : 0xffffffff.

## 0x8056f0 RxClusterSetAttributes  (catalogue: specified/REA, port level 0)
- `RxCluster* (RxCluster* c, RwUInt32 attr)`: if c: `c+0x18 = attr`, return c; else NULL.

## 0x8059c0 RxPipelineNodeCloneDefinition  (catalogue: specified/REA, port level 2)
- `RxNodeDefinition* (RxPipelineNode* node, RxClusterDefinition* cd)`. node NULL -> NULL. cd NULL, or node's nodeDef has no clusters of interest,
  or cd is not among them -> return `0x805a20(node, cd)` (clone, adding cd when non-NULL; 0x805a20 not specified in this pass). cd already among
  the clustersOfInterest -> NULL (0x8059ff, EAX still 0).

## 0x805d10 RxPipelineNodeFindOutputByName  (catalogue: specified/REA, port level 0)
- `RwUInt32* (RxPipelineNode* node, const RwChar* name)`: NULL when node, node->nodeDef or name is NULL. For i = 0 .. `def->io.numOutputs (+0x2c)`-1
  (signed loop, 0x805d34): if `vecStrcmp (Engine+0x114)(def->io.outputs[i].name (+0x30 array, stride 0xc, name at +0), name) == 0` return
  `&node->outputs[i]`. Not found -> NULL.

## 0x805d70 RxPipelineNodeFindOutputByIndex  (catalogue: specified/REA, port level 0)
- `RwUInt32* (RxPipelineNode* node, RwUInt32 index)`: node && node->nodeDef && `index < node->numOutputs` (unsigned) -> `&node->outputs[index]`; else NULL.

## 0x805da0 RxPipelineNodeFindInput  (catalogue: specified/REA, port level 0)
- `RxPipelineNode* (RxPipelineNode* node)`: returns node when node and node->nodeDef are non-NULL, else NULL.

## 0x805f30 RxPipelineNodeGetInitData  (catalogue: specified/REA, port level 0)
- `void* (RxPipelineNode* node)`: returns `node+0x20` initializationData (no NULL check).

## 0x806bc0 RxPipelineFindNodeByIndex  (catalogue: specified/REA, port level 0)
- `RxPipelineNode* (RxPipeline* pipe, RwUInt32 index)`: pipe && `index < numNodes` (unsigned) -> `pipe->nodes + index*0x28`; else NULL.

## 0x807040 RxLockedPipeDeleteNode  (catalogue: specified/REA, port level 1)
- `RxPipeline* (RxPipeline* pipe, RxPipelineNode* node)`: requires pipe, `pipe->locked`, node and `node->nodeDef` non-NULL/non-zero, else NULL;
  `0x8050c0(node, pipe)`; returns pipe.

## 0x807070 RxLockedPipeSetEntryPoint  (catalogue: specified/REA, port level 0)
- `RxPipeline* (RxPipeline* pipe, RxPipelineNode* node)`: requires pipe, locked, node, node->nodeDef; `k = (node - nodes)/0x28` (signed /40);
  requires `nodes + k*0x28 == node`, `k < numNodes`, `k != -1`; then `entryPoint (+0x28) = k`, return pipe; else NULL.

## 0x8070d0 RxLockedPipeGetEntryPoint  (catalogue: specified/REA, port level 0)
- `RxPipelineNode* (RxPipeline* pipe)`: NULL unless pipe && locked && `entryPoint != -1` && `entryPoint < numNodes`; else `&nodes[entryPoint]`.

## 0x807100 RxLockedPipeAddPath  (catalogue: specified/REA, port level 0)
- `RxPipeline* (RxPipeline* pipe, RwUInt32* output, RxPipelineNode* target)`: requires pipe, locked, output, `*output == -1`, target, target->nodeDef,
  and the exact-index test of 0x807070; then `*output = index`, return pipe; else NULL (no error set).

## 0x807170 RxLockedPipeDeletePath  (catalogue: specified/REA, port level 0)
- `RxPipeline* (RxPipeline* pipe, RwUInt32* output, RxPipelineNode* target)`: requires pipe, locked, output, target; if
  `pipe->nodes + (*output)*0x28 == target` then `*output = 0xffffffff` and return pipe (EAX unchanged); else NULL.

## 0x80ea40 RxRenderStateVectorSetDefaultRenderStateVector  (catalogue: specified/REA, port level 1)
- `RxRenderStateVector* (RxRenderStateVector* rsv)`. NULL -> `_rwerror(0x80000016)`, RwErrorSet, NULL.
- If `Engine+0x150 engineStatus == 3` (started): copy 11 dwords from the module default `P+4` into rsv (0x80ea75); return rsv.
- Else if rsv != P+4: `_rwerror(0x80000018)`, RwErrorSet, NULL (only the module default may be initialised before start).
- Else fill (0x80eacb-0x80eaf8): +0x0 Flags = 7, +0x4 ShadeMode = 2, +0x8 SrcBlend = 5, +0xc DestBlend = 6, +0x10 TextureRaster = 0,
  +0x14 AddressModeU = 1, +0x18 AddressModeV = 1, +0x1c FilterMode = 2, +0x20 BorderCol = 0xffffffff, +0x24 FogType = 0,
  +0x28 FogCol = 0xffffffff; return rsv.

## 0x80f070 _rxPacketDestroy  (catalogue: specified/REA, port level 3)
- `void (RxPacket* pkt)`. `pkt->pipeline->embeddedPacketState (+0x10) = 1` (unused). For each of `numClusters` (u16 at +2) clusters
  (stride 0x1c from +0x14), do-while loop: if `clusterRef (+0x14)`: free data via RxHeapFree when data && !(flags & 2); then zero the
  dword at +0 (flags AND stride), data, numAlloced, numUsed, clusterRef. Finally `pkt->flags (u16) = 0`.
- ORIGINAL BUG: the loop is a do-while with `dec edi; jne` (0x80f0bf), so numClusters == 0 would iterate 2^32 times (never happens
  for unlocked pipelines with clusters).

## Generated sections (byte-identical shapes, see _generated-p3.md for the method)

## 0x804fc0 RxPipelineSetFreeListCreateParams  (catalogue: specified/REA, port level 0)
- Shape group: SetFreeListCreateParams (18 byte-identical functions; generated from the bytes 0x804fc0-0x804fd3). Callers: none in callgraph (reached through data / function pointer).
- `void (RwInt32 blockSize [esp+4], RwInt32 numBlocksToPreallocate [esp+8])`, cdecl, `ret`. Stores blockSize -> [0x8e26a0] (0x804fc8) and numBlocksToPreallocate -> [0x8e26a4] (0x804fcd); both s32 globals (the module free-list creation parameters; their reader is not part of this function). Image defaults: [0x8e26a0] = 64, [0x8e26a4] = 1. EAX = blockSize on return (RW 3.6 declares void). No validation.
- librw: absent (librw has no free-list create params; it uses malloc per object)

## 0x8052f0 RxHeapGetGlobalHeap  (catalogue: specified/REA, port level 0)
- Shape group: global getters (4 byte-identical functions; generated from the bytes 0x8052f0-0x8052f5). Callers: 0x7ef6b0 RwIm3DRenderPrimitive, 0x7ef810 RwIm3DRenderTriangle, 0x7ef900 RwIm3DRenderLine.
- `u32 ()`, cdecl: EAX = [0xc9a6b0] (dword global, 0x8052f0).
- librw: absent (RW-internal boilerplate)

## 0x8055c0 RxClusterUnlock  (catalogue: specified/REA, port level 0)
- Shape group: empty functions (7 byte-identical functions; generated from the bytes 0x8055c0-0x8055c0). Callers: none in callgraph (reached through data / function pointer).
- `void ()`: a single `ret` (cdecl, no stack args popped); no effect.
- librw: absent (RW-internal boilerplate)

## 0x807c70 sub_807c70 (proposed: _rwPipeAttachDummy)  (catalogue: unknown/REA, port level 0)
- Shape group: ValidatePlugins / TRUE stubs (14 byte-identical functions; generated from the bytes 0x807c70-0x807c75). Callers: 0x7f3170 RwEngineInit.
- `RwBool (void*)`: returns 1 (TRUE) in EAX, cdecl `ret`; no argument read, no side effects (release build stub).
- librw: absent

## 0x80e990 RxNodeDefinitionGetD3D9ImmInstance  (catalogue: specified/REA, port level 0)
- Shape group: constant getters (4 byte-identical functions; generated from the bytes 0x80e990-0x80e995). Callers: 0x80a0a0 _rwIm3DCreateTransformPipeline.
- `void* ()`, cdecl: EAX = 0x8e2918 (immediate address/constant loaded at 0x80e990).
- librw: absent (RW-internal boilerplate)

## 0x80e9a0 RxNodeDefinitionGetD3D9SubmitNoLight  (catalogue: specified/REA, port level 0)
- Shape group: constant getters (4 byte-identical functions; generated from the bytes 0x80e9a0-0x80e9a5). Callers: 0x80a1a0 _rwIm3DCreateRenderPipelines.
- `void* ()`, cdecl: EAX = 0x8e2978 (immediate address/constant loaded at 0x80e9a0).
- librw: absent (RW-internal boilerplate)

## 0x805820 _rxPipelineDestroy  (catalogue: specified/REA, port level 1)
- `void (RxPipeline* pipe [esp+4])`. NULL -> return. `n = numNodes` captured once (0x805835); for i = 0..n-1 call `0x8050c0(nodes + i*0x28, pipe)`
  walking forward with a fixed stride (0x805841). For an unlocked pipeline 0x8050c0 does not compact, so every node is destroyed; for a LOCKED
  pipeline 0x8050c0 compacts the array after each call, so this loop skips every other node and walks past the live nodes (original
  behaviour; callers destroy unlocked pipelines).
- `nodes (+8) = 0`; if `superBlock (+0x20)`: free it (Engine+0x138), `+0x20 = 0`, `+0x24 = 0`. `Engine+0x148(P->freeList, pipe)` returns the
  0x34-byte object to the pipeline free list (0x805884).

## 0x805a20 sub_805a20 (proposed: _rxNodeDefinitionClone; rw-names hint _rxPipelineNodeClustersRealloc)  (catalogue: unknown/REA, port level 1)
- `RxNodeDefinition* (RxPipelineNode* node [esp+4], RxClusterDefinition* cd [esp+8])`: builds an editable copy of `old = node->nodeDef` in one
  malloc block, appending cd as an extra cluster of interest when cd != NULL. Callers RxPipelineNodeCloneDefinition, RequestCluster,
  ReplaceCluster.
- `n = old->numClustersOfInterest + (cd ? 1 : 0)`; `nameSz = (strlen(old->name) + 1 + 3) & ~3`; `nOut = old->io.numOutputs`;
  size = `0x40 + nameSz + 12n + 4n + 12*nOut + sum_j((strlen(out_j.name) + 1 + 3) & ~3) + nOut*4n` (0x805a43-0x805ab5, all rounded up to 4).
  `p = Engine+0x134(size, 0x30409)`; NULL -> `_rwerror(0x80000013, size)`, RwErrorSet, NULL.
- Layout written in this order (cursor starts at p+0x40): name string (`new->name = p+0x40`, byte copy incl. NUL); nodeMethods 7 dwords copied
  from old+4 (0x805b0f); `+0x20 = n`; `+0x24 clustersOfInterest = cursor` (12n), old entries copied; if cd: entry[old n] = {cd, 0, 0};
  `+0x28 inputRequirements = cursor` (4n), old values copied (dword+byte copy), if cd: last = 0; `+0x2c = nOut`; `+0x30 outputs = cursor`
  (12*nOut); for each output j: `name = cursor` (string copied), `outputClusters = cursor + rounded name size` (4n): old values copied, if cd
  the last = old `allOtherClusters (+8)`; `allOtherClusters` copied.
- `+0x34 privateDataSize = old's`; `+0x38 editable = 1`; `+0x3c InputPipesCnt = 0`. If `old->InputPipesCnt == 0 && old->editable` free(old).
  `node->nodeDef = new`; return new. (old->InputPipesCnt is not decremented.)

## 0x805db0 RxPipelineNodeRequestCluster  (catalogue: specified/REA, port level 2)
- `RxPipeline* (RxPipeline* pipe, RxPipelineNode* node, RxClusterDefinition* cd)`: NULL unless pipe, `pipe->locked`, node, node->nodeDef and cd are
  set; NULL if cd is already a cluster of interest. `def = 0x805a20(node, cd)`; NULL -> NULL; `def->inputRequirements[def->numCOI - 1] = 2`
  (rxCLREQ_OPTIONAL, 0x805e06). Returns pipe.

## 0x805e20 RxPipelineNodeReplaceCluster  (catalogue: specified/REA, port level 2)
- `RxPipeline* (RxPipeline* pipe, RxPipelineNode* node, RxClusterDefinition* oldCd, RxClusterDefinition* newCd)`: requires all four (and locked,
  nodeDef); i = index of oldCd in the clustersOfInterest; not found -> NULL. `def = 0x805a20(node, NULL)`; NULL -> NULL;
  `def->clustersOfInterest[i].clusterDef = newCd` (0x805e85); return pipe. newCd is not checked for duplicates.

## 0x805ea0 RxPipelineNodeCreateInitData  (catalogue: specified/REA, port level 1)
- `void* (RxPipelineNode* node, RwUInt32 size)`: frees an existing `+0x20` block (and zeroes +0x20/+0x24); `p = Engine+0x134(size, 0x30409)`;
  `+0x20 = p`; NULL -> `_rwerror(0x80000013, size)`, RwErrorSet, return NULL; else `+0x24 = size`, return p. No NULL check of node.

## 0x806ac0 RxPipelineClone  (catalogue: specified/REA, port level 5)
- `RxPipeline* (RxPipeline* src)`: NULL if src is NULL or locked. `q = RxPipelineCreate()`; NULL -> NULL. `q->pluginId = src->pluginId`,
  `q->pluginData = src->pluginData`. If `src->numNodes == 0` return q (unlocked, empty). Else `RxPipelineLock(q)`, `0x8065e0(q, src)`,
  `RxLockedPipeUnlock(q)`; any 0 -> `_rxPipelineDestroy(q)`, NULL. Returns q. entryPoint of q stays 0 (the source's is not copied).

## 0x806f20 RxLockedPipeReplaceNode  (catalogue: specified/REA, port level 2)
- `RxPipeline* (RxPipeline* pipe, RxPipelineNode* node, RxNodeDefinition* def)`: requires pipe, locked, node, node->nodeDef, def, else NULL.
- `nn = &nodes[numNodes]`; `0x806de0(pipe, nn, def)`; 0 -> NULL. `m = min(nn->numOutputs, node->numOutputs)`; copy m output indices from node to
  nn. If `m < nn->numOutputs`, the entries m .. nn->numOutputs-1 of the OLD node's outputs are set to 0xffffffff (0x806f9f: writes through
  `node->outputs`, not nn's; nn's are already -1 from 0x806de0 - harmless original slip).
- `newIdx = (nn - nodes)/0x28`, `oldIdx = (node - nodes)/0x28`; every output of every node equal to oldIdx becomes newIdx (0x806ff5-0x807011).
- `0x8050c0(node, pipe)` (locked: compacts and renumbers). Returns pipe. entryPoint is not adjusted (if node was the entry point, the entry
  index afterwards refers to whatever node moved into that slot).

## 0x8071b0 RxPipelineInsertDebugNode  (catalogue: specified/REA, port level 5)
- `RxPipeline* (RxPipeline* pipe [esp+4], RxPipelineNode* before [esp+8], RxPipelineNode* after [esp+0xc], RxNodeDefinition* debugNode [esp+0x10])`.
  Never called by the game (debug API); specified for completeness.
- Validation (all failures return NULL without an error): pipe NULL; before and after both NULL; debugNode NULL. before == NULL requires
  `after == pipe->nodes` (0x8071eb). before != NULL must be an exact node of pipe with index < numNodes (0x8071f9-0x807220). after == NULL
  requires `before->numOutputs == 0`; else after must be an exact node (afterIdx). If both: k = first output of before with
  `outputs[k] == afterIdx` (0x807286); none -> NULL.
- `list = Engine+0x134(pipe->packetNumClusterSlots * 8, 0x10409)`; NULL -> `_rwerror(0x80000013, slots*8)`, RwErrorSet, NULL. Collect the
  distinct cluster definitions `before->slotClusterRefs[j]->clusterRef` (j < slots, non-NULL refs), then the same for after (0x8072f9-0x807379),
  4-byte entries (the buffer is twice the needed size).
- `RxPipelineLock(pipe)`; NULL -> NULL (the list buffer leaks). `p2 = RxLockedPipeAddFragment(pipe, &first, debugNode, NULL)` (result stored over the
  debugNode argument slot); `nn = p2->nodes + first*0x28` (no NULL check of p2: crash if AddFragment failed). For each collected cd: if
  `pipe->locked && nn && nn->nodeDef && cd` and cd is not yet a cluster of interest of nn: `d = 0x805a20(nn, cd)`; if d:
  `d->inputRequirements[last] = 2` (inlined RequestCluster).
- Re-wiring (inlined Delete/AddPath, using p2): if before: `out = &before->outputs[k]` (when nodeDef and k < numOutputs); if it points at `after`
  set it to -1 (0x80745e); then if `*out == -1` and nn is valid, `*out = index(nn)`. If after: `o0 = &nn->outputs[0]` (when nn has outputs);
  if `*o0 == -1` and after->nodeDef and after is an exact node, `*o0 = afterIdx`.
- `r = RxLockedPipeUnlock(p2)`; r NULL -> `_rxPipelineDestroy(p2)` (the whole pipeline is destroyed on failure). Free the list; return r.
- Note: before/after are raw node pointers taken before RxPipelineLock; they stay valid only because Lock does not reallocate when the superblock is
  already large enough (always true after a successful unlock).
