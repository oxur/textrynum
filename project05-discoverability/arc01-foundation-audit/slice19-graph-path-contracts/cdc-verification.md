# CDC verification — slice19 graph path contracts

All five PTH criteria independently accepted as a bounded STATIC API assessment
at author30de32d61f80e2f5a6e6cc96b461f98686be185e. No new severity-graded
finding supported by the inspected contracts; four ungraded owner questions
retained. This is not global graph correctness/exoneration or runtime acceptance.
Same retained LunaHigh author/Sol6.1High CDC, no unavailable reviewer credited.

## Own source and actual intake

CDC own current reads algorithms1–275/770–860, types20–140/170–435/435–590,
MCPtools90–113/180–230/385–422/680–745. Followup independently read query40–122/
240–347 and tools495–535/875–900. Actual pinned petgraphA*1–199 and Graph insertion/
findedge620–685/1050–1115 directly read. Earlier14 actualsameCC full4446line
receipt rechecked: types1–320/321–640/641–969; algorithms1–300/301–610/611–930/
931–1200/1201–1377; tools1–350/351–700/701–1050/1051–1400/1401–1750/1751–2100.
Frozen14log690752bytes SHA2565caa7619a1d049f1e78ad2eab7c939ab6f79aa1d411b526ab4b0ae01f3f62094.
This is reuse of actual unchanged own-author intake, not another agent's receipt.

19actualsource commands independently reconciled freshfullaffectedunits: types
20–145/225–360/337–430/361–510/431–510, algorithms1–75/210–285/775–865, MCP
375–422/685–750 plus requiredargs/publicmetadata and edgefollowup ranges; query
38–70/90–130/238–275/295–350. CompletepetgraphA*1–210 throughactual199 plus
Graph615–690/1050–1105. Allsource readcommands0; exactunits recovereddirectly,
no new4446line restart. Citedpre-editreadback supported. Fullguidance retained.

Cachedpetgraph0.8.3 archive checksum8701b58ea97060d5e5b155d383a69952a60943f0e6dfe30b04c287beb0b27455
matches productlock7739–7744. CDC compared whole selectedcachedfiles byte-equal
to archive: astar.rsSHAf87d74da93e9a80e93e5b71eaaf14739ff818f7bc534596b46fdfb628702bab8;
graph_impl/mod.rsSHA266f282966354291683264d1f481a81bdd02c70159ba66c613a5fa91a0f9d923.
Semantic reading is narrow selecteddependency API/implementation only, not whole
petgraph/thirdparty audit. No Cargo/fetch/algorithm execution.

## Independent interpretation and negative checks

Exactdocs say 'Uses petgraph's A* algorithm with uniform cost.' Uniform-cost
search can optimize weighted costs, and zeroheuristicA* matches that method.
Wording alone is not an equaledge/unitcost/minimumhop promise. Pre-issue19plan
premature 'conflicts' wording was corrected before prompt commit/dispatch. No
false documentation finding is inferred from that phrase. Weighted intent also
appears in Relationship defaultweight/Edge docs; domain interpretation remains
underconstrained in the inspected units.

Manual finite1/.5 arithmetic checked: two.5edges cost4/rawsum1/hops2 versus
three1edges cost3/rawsum3/hops3. Currentoptimizer favors cost3, without implying
rawsum/hopmetric shouldwin. A single.5edge gives optimizer2/rawsum.5; their unequal
values alone are no defect. Validnodes preexist, uniqueIDs, directednormalpublic
construction, no corrupted graphstate or unsupported weights.

PetgraphA* stores predecessorNODES, not edgeIDs. FABRYK shortest_path then calls
find_edge for each consecutive endpointpair and sums returned rawweights. New
parallel edges are adjacencyhead; find_edge returns firsttargetmatch. Thus with
1edge then.5edge, reportedrepresentative.5; reverseinsertion gives1, while best
internalA*cost1 remains. This is static bookkeeping, not observed algorithmoutput.
PathResult says edgesalongpath; inspectedunits do not state chosenedgeprovenance
or deterministicties. Both representative edges join the returnednodes. Keep the
selected-versus-representative contract an owner question, not an invented guarantee.

Own MCPschema/handler/queryreads confirm nodeIDs, relationship_to_next, nodecount
length and rawtotalweight projection. Supported graph_get_node_edges returns all
incident candidate endpoint/relationship/rawweight information. It supplies useful
followup without identifying a prior optimizer edge/snapshot. A slim payload or
omitted internalcost/edgeID alone is not a defect. No actual client/ChatGPT trial.
Equalcostparallel/tiedroutes, singleedge, missing/samenode/reversedirection controls
support limits. Accepted038direction is independent and unchanged. No stronger
promise found within THIS inspectedscope; remainingwholegraph/docs audit is open.

## Criterion walk and owner handoff

| ID | Independent evidence | Verdict |
| --- | --- | --- |
| PTH-01 | Substantiated14full4446receipt plus actual19freshcompleteunits/pinnedreference/guidance | Accepted bounded intake |
| PTH-02 | Exactwording/actualcost/rawsum/resultpolicy/MCPfollowup separated | Accepted static contract map |
| PTH-03 | Validfinite arithmetic/parallelinsertion and ties/single/missing/same/direction controls | Accepted manualderivation only |
| PTH-04 | No gradedfinding; four actionable underconstrainedpolicyquestions, no invented metric/provenance | Accepted calibration |
| PTH-05 | Exactfiveauthorpaths, pre493prefixes, protected9byteidentical, clean719, actualbothauthors | Accepted preservation/own commit |

Ungraded questions, not mandatory feature repairs:
- Q19-01 metricmeaning: graphAPI/domainowner chooses documented semantics or keeps
  current unspecifiedpolicy; future oracle follows that choice using finitecases.
- Q19-02 representativeversusoptimizeredge: graph/MCPAPIowners decide guarantee;
  future insertion/tie cases verify only selectedcontract, no assumed edgeIDfield.
- Q19-03 rawtotal_weight meaning: sameowners clarify rawsum versus optimizercost,
  possibly documenting currentfield rather than adding payload; future source/
  response examples must match chosen semantics.
- Q19-04 tiepolicy: graphAPIowner may leave choices unspecified; only if a stable
  guarantee is adopted should later oracle require a specific equalcost result.
No policy chosen, no behavioral oracle run or commissioned; no newFA-RS ID allocated.

## Preservation, record correction and actual handback

Exec82382completed0 and returnedfivepaths/index; noactiveownedmutablecommand.
Actual30de both normalized coauthorvalues parse. Commit exactly artifact/report/
index/ledger/close. CDC independently compares complete pre493c7a7 report/index
prefixes and protected9bytes unchanged. Hashing/comparison readsbytes only; no
textual/semantic diagnosticinspection/reconciliation/execution. UnfinishedCDC
quartet stillUNVERIFIED, SEARCH unchanged, source/index719clean, privacyownedpaths/
content0. No nonzero author commandexits; no source/algorithm/test/native/service
operation. Frozen19log330205bytes SHA2563cadb19d6e0582f9155711b4138bf37fd3e4c3e34bf71f16e3cd8ffe01ee84a7,
/private/tmp/textrynum-graph-path-slice19-run.jsonl, encodedprivatepattern0.

CDC corrects one report boundary attribution: petgraph tracks nodes; Fabryk uses find_edge reconstruction; petgraphA* returns node predecessors. Companion already separated them. Also
qualify uniformwording as a possible methodreference and guarantees as inspected-
scope limits; add stablequestionowners/oracles. No extra authorround/refinement.
19budget0/5;18/17/16:0/5;15:1/5;03:2/5 unchanged. All001–043/38/31/291 owners,
49cells7/7/35, SEARCH/fullpasses/finalindependentcomposition unchanged. Arcactive.
