# Resource scheduling declaration and enforcement contracts

Status: bounded static scope recorded before author work; initial0/5 refinements.
Same retained LunaHigh author/Sol6.1High CDC. Source/index719 read-only.

## Concrete population and own readiness

Primary full author files: topo schedule.rs451, resource_graph.rs214 and spec
stage.rs130:795lines including all tests. Complete companion logical units:
topo resolve1–116/publicresolver, lib1–100/publictopology, traits210–282/Stage+
StageContext; spec validation1–34/validate, defaults1–34/concurrency declaration;
runner1–270/construction-run-batch,398–445/condition-selection-context; batch1–289/
complete non-test execution+retry implementation. Recover a boundary if an actual
unit extends farther; no unrelated whole-file acceptance from these selected reads.

CDC own current reads primary three files complete795lines; resolve1–150, topolib
1–135, traits210–310, specvalidation1–85/defaults1–45, runner1–270/390–467 and
batch1–307. Actual full source units support preliminary current call map; not
the unavailable composition reviewer's intake. No execution occurred. Guessed
per-crate README paths were absent (rg ENOENT); they supply no supported promise.

## Actual contract question, not a held feature commission

Current ResourceSpec writes says exclusive access. Public schedule docs promise
no within-batch write conflicts; ResourceGraph collects writers but schedule
parameter _writers is unused and final grouping uses only creator-reader depth.
Preserve ResourceGraph15–18's future-milestone/model-only comment as explicit
contrary/scope evidence, beside current publicresolve94–99 and runner consumers.
A current API/doc mismatch may justify clarifying/rejecting unsupported declarations
rather than implementing project06 scheduling features. No historical plan grants
execution promises; original agent workflow cancelled, project06 features held.

Before grading trace CURRENT validity: specvalidate, duplicatecreator/writer checks,
exact resource-name/version identity, derived creator-reader edges, cycle checks,
registeredhandler/source preconditions and current public callability. Then inspect
all selected downstream enforcement: runnerstage spawning, condition/input gates,
perstage/global semaphores or resource locks, batch/nonbatch helpers, handler/caller
responsibility. Depth grouping alone is not proof of overlapping operations.

Manual case uses two unique valid stage names writing the SAME exact external
resource string, no creators/reads for that resource; satisfy nonempty source/stage
and valid source refs/registeredhandlers in stated assumptions. Derive accepted
resource map, edges/depth/batches; compare shared-read-only, distinct resources,
creator-before-reader and deliberately serialized/manual/handler-lock controls.
Consider one creator feeding two same-resource writers as a second source case,
not necessary to invent a conflict if validation rejects it. Distinguish planner
output promise from source-permitted concurrent dispatch and from actual handler
overlap/dataeffects (unobserved). Defaults.concurrency1 is a potential control,
not assumed global: inspect actual semaphore lifetime/location. No runtimecheck.

No generic concurrency finding/repair by count target. If scope/contracts remain
uncertain retain questions. Stable043+ only if supported, with current promise/
contrary comment/owner compatibility and later disposition choices. All results
project05artifacts; five authorpaths/index in prompt. Preserve all prior001–042,
original38/31/291 owners and49cells7/7/35, SEARCH/fullpasses/finalcomposition.
