# World Address Space: A Systems Primitive for Autonomous Software

**Status:** research discovery, not a product proposal  
**Method:** landscape → shared assumptions → hypothesis → prior-art kill → refinement  
**Date:** 2026-09-16  

This note records a search for a *new systems abstraction* for autonomous software. It is not a survey of agent frameworks, not a startup brief, and not a feature list. The question is:

> What should an autonomous AI system actually be at the systems level?

The search was conducted against academic papers, systems conferences, GitHub, commercial products, and adjacent OS / database / virtualization / PL literature. mesh-llm application source was deliberately not consulted.

The surviving candidate is a **World Address Space**: a typed, content-addressed, copy-on-write page table whose pages include filesystem chunks, process heaps, KV-cache blocks, capabilities, and an effect log. The schedulable entity is the world, not an “agent.” Language models attach as untrusted proposal devices. The only mutation interface is a typed world-delta ISA. Fork, migrate, checkpoint, and replicate are page-table operations.

Novelty is **not claimed as absolute**. Classification: **partially explored in pieces, mostly unexplored as a single ABI.** Closest systems (Arbiter-K, ProcessFork, DeltaBox, PatchBoard, InferNode, Croquet, thaw) each implement one slice and keep the classical agent loop as the control plane.

---

## 1. Method

The procedure was:

1. Map the current agent stack and the runtimes that claim to replace it.
2. Extract the architectural assumptions they share, even when their marketing disagrees.
3. Generate hypotheses that break those assumptions.
4. Search prior art under many names.
5. Kill anything that is “existing technology + LLM,” or an OS metaphor wrapped around ReAct.
6. Refine what remains and search again.
7. Keep one candidate if it still has a missing primitive, a prototype path, and a falsifying experiment.

A lack of hits on one query was never treated as novelty. Several “new” ideas died on the second or third terminology expansion.

---

## 2. Landscape: what autonomous software is today

### 2.1 The dominant execution model

Across research prototypes, commercial SDKs, coding agents, and computer-use systems, execution converges on one interpreter:

```text
model  →  tokens in a context window  →  tool/function call
       →  environment observation      →  model  →  next action
```

Names differ (ReAct, function calling, planning, reflection, workflow graphs, event-driven agents). The control flow does not. The model is the sequential controller. The environment is a peripheral accessed through an RPC-shaped ABI called a “tool.” State that does not fit in the context window is stuffed into a side store labeled “memory.”

### 2.2 Agent runtimes and their shared skeleton

| System | What it adds | What it does not change |
|---|---|---|
| LangGraph | Durable graph + checkpoints of Python state | Tool-calling loop; environment is outside the graph |
| AutoGen / CrewAI / OpenAI Agents SDK | Multi-actor roles, handoff, structured tools | Same loop, messages as IPC |
| Anthropic computer-use / browser agents | Desktop or DOM as a tool | Environment is a screenshot/action API, not a world object |
| OpenHands / SWE-agent | Repo + shell + tests as tools | Sandbox is a container the agent talks to |
| E2B / Daytona / Modal / AgentENV | Fast sandboxes, microVM snapshots | Environment fork without model/KV fork |
| Temporal / Restate / DBOS + Pydantic AI | Journal of model and tool steps | Durable *replay of the loop*, not a world |
| MCP | Tool/resource transport | Still tools |
| A2A / ANP / ACP | Agent cards, task RPC, DIDs | Still messages between agent processes |

Independently, these systems all treat:

1. An agent as a *conversation plus a tool loop*.
2. State as *messages*, optionally checkpointed.
3. The environment as *someone else’s process*.
4. Other agents as *RPC peers*.
5. Memory as a *database next to the loop*.

That is the architectural monoculture.

### 2.3 Adjacent “agent OS” wave (2024–2026)

A large 2026 literature puts OS vocabulary on the same loop:

- **AIOS** (Mei et al., arXiv:2403.16971): LLM kernel, agent syscalls, context switching across agents.
- **AOS** (arXiv:2606.01508): agentic control plane inside or beside a traditional OS.
- **Agent libOS** (arXiv:2606.03895): `AgentProcess`, capability-checked primitives, tools as libc wrappers.
- **Astrid**, **Rivet agentOS**, **jordanhubbard/agentos** (seL4), **InferNode** (Inferno/9P): user-space kernels, WASM/V8 guests, namespaces.
- **LLMOS** (MikeyBeez and related essays): the LLM *is the CPU*; the kernel fetches/decodes/executes “intents.”
- **Arbiter-K** (Wen et al., arXiv:2604.18652): LLM demoted to a Probabilistic Processing Unit; Semantic ISA; taint; kernel validates sinks.

These are real systems work. They are not a new process model. They wrap ReAct in a syscall table.

---

## 3. Shared assumptions (the things everyone independently rebuilds)

The interesting signal is not any one product. It is that *unrelated* groups converge on the same five assumptions.

### A1. The agent is the process

The schedulable, named, checkpointed entity is “an agent.” The world it mutates is attached state.

### A2. The language model is the controller

Even when a paper says “the LLM is untrusted,” the sequential program counter is still “ask the model what to do next.” Arbiter-K and LLMOS make this explicit: the model is a CPU.

### A3. Tool calls are the ABI

Whether branded as functions, MCP tools, syscalls, or 9P writes to `/tool`, the interface is “name + JSON arguments + observation text.” The environment is not typed into the agent’s address space.

### A4. Cognitive state and environment state are different objects

Conversation / KV cache / “memory DB” live in one plane. Files, processes, browsers, credentials live in another. Rollback of one without the other is considered normal. Coordinated restore is a feature bolted on later.

### A5. Communication is messages

Agents send tasks, texts, or artifacts to each other. They do not ship attenuated views of a world, and they do not replicate computation against a shared timebase.

If these five are wrong, optimizing the loop is the wrong research program.

---

## 4. Kill log

Each subsection is: hypothesis → search → prior art → verdict → what it taught.

### K1. “An operating system for agents”

**Hypothesis:** Linux-like process/scheduler/syscall/FS abstractions, but the process is an LLM agent.

**Prior art:** AIOS, AOS, Agent libOS, Astrid, Rivet agentOS, seL4 agentOS, InferNode, LLMOS, “Agentic Microkernel” industry whitepapers.

**Verdict: REJECT.** The category exists and is already crowded. Most implementations are user-space runtimes that rename tool calls to syscalls.

**Lesson:** OS vocabulary is not an OS primitive. A primitive has to change what is scheduled and what a mutation is.

### K2. “Git for agents” (conversation trees, trajectory branches)

**Hypothesis:** Branch/merge agent execution like Git.

**Prior art:** LangGraph `update_state` forks; AgentGit (arXiv:2511.00628); AIVCS; convex-branch-agent; AgentStateGraph; Shepherd’s Git-like execution traces (arXiv:2605.10913).

**Verdict: REJECT** as a conversation/trace VCS. Merging natural-language trajectories is not a well-defined operation; systems fall back to LLM-as-judge “semantic merge.”

**Lesson:** Branching is only technically meaningful if the *state object* has a mechanical diff. Text traces do not.

### K3. “Snapshot the KV cache; that is the agent”

**Hypothesis:** Portable agents = portable KV + weights.

**Prior art:** thaw (vLLM/SGLang session freeze/branch/checkout); kvpolicy; online KV migration benches; ForkKV (CoW disaggregated KV for multi-LoRA); ObjectCache; LMCache / Dynamo-style KV tiers.

**Verdict: REJECT** as the whole primitive. Necessary, not sufficient. KV is working memory of *one model attachment*, not the agent-environment object.

**Lesson:** KV pages should be a *page type* in a larger address space, not the definition of the process.

### K4. “Compile the agent loop” (LLVM for agents)

**Hypothesis:** Agent program → IR → optimized execution graph, replacing interpreted ReAct.

**Prior art:** LLMCompiler; AgentIR; APXM (MLIR AIS dialect); CompileAgent; Agentus bytecode VM; DSPy compilation; Pangolin (effects + selection monad).

**Verdict: REJECT** as a replacement architecture. Compiling *plans* still assumes the loop; it just lowers some of it. Useful as an optimization, not a new process model.

### K5. “Agent network protocol”

**Hypothesis:** TCP/IP for agents: `discover`, `delegate`, `migrate`.

**Prior art:** MCP, A2A, ANP, ACP, AG-UI, AP2; 1990s mobile agents (Telescript strong mobility, IBM Aglets weak mobility).

**Verdict: REJECT.** A2A is HTTP task RPC plus agent cards. 1990s mobile agents already tried “the agent moves.” They failed on heterogeneous runtimes, security, and the fact that *the interesting state was never just the program counter*.

**Lesson:** If mobility is real, what moves cannot be “the agent process.” It has to be a smaller, typed state object. Weak vs strong mobility is still the right dichotomy.

### K6. “Algebraic effects for tool calls”

**Hypothesis:** Tools are effects; handlers sandbox, replay, speculate, or deny.

**Prior art:** Pangolin; “Composable Effect Handling for LLM-integrated Scripts” (arXiv:2507.22048); Etas (arXiv:2607.17780); Zylos notes; Shepherd’s reversibility tiers.

**Verdict: Closely explored → probably reject** as the *top-level* primitive. Effects are the right *language* for intercepting mutations. They do not specify what the durable object is.

**Lesson:** Keep effect handlers as the kernel’s interpretation strategy. Do not stop at “effects instead of tools.”

### K7. “Durable execution”

**Hypothesis:** Make the agent a Temporal/Restate workflow.

**Prior art:** Pydantic AI durable backends (Temporal, DBOS, Prefect, Restate); Restate journals LLM calls and tool steps.

**Verdict: REJECT** as the process model. This is crash recovery for the interpreter. It does not unify environment and cognition, and it does not make fork cheap.

### K8. “Capability security / 9P namespaces”

**Hypothesis:** Capabilities instead of credentials; tools as filesystem paths.

**Prior art:** Agent libOS; Capsicum/seL4 mappings in CISPA-style blueprints; Zylos OCap notes; namespace-bounded 9P agents; InferNode/Veltro (LLM as `/n/llm` files); WASM component model + WIT (Astrid, AgentBox, Wassette).

**Verdict: Closely explored** as a *security model*. InferNode is the most architecturally honest: ambient authority dies because the path does not exist.

**Lesson:** Namespaces and capabilities are how views of a world should be granted. They are not themselves a world.

### K9. “Transactional agents / STM / sagas”

**Hypothesis:** ACID or STM over agent actions.

**Prior art:** Cordon (semantic transactions, shadow state, effect outbox, arXiv:2606.17573); Agentic Transaction (arXiv:2608.13900); ATCC (adaptive CC for unforeseen SQL-issuing agents); position paper “MAS should prioritize concurrency control” (arXiv:2608.18092); agent-saga compensations; PatchBoard JSON Patch kernel (arXiv:2605.29313).

**Verdict: Partially explored.** Transactions over *tools* or *JSON boards* exist. STM’s classical ban on I/O inside transactions is exactly the agent problem: almost everything is I/O.

**Lesson:** Isolation has to be over a *shadow world*, not over a lock table of tool names. PatchBoard is the closest: agents propose patches; a kernel commits. But the state is a schema, not an address space.

### K10. “Millisecond sandbox fork”

**Hypothesis:** The missing OS primitive is fast C/R of files + processes.

**Prior art:** DeltaBox / DeltaState (arXiv:2605.22781): DeltaFS overlay layer freeze + DeltaCR CRIU/template-fork, ~ms rollback, hides checkpoint in the LLM wait; Firecracker snapshots; PandaStack; forkd; AgentENV; AgentFS (SQLite VFS); Agent Harbor AgentFS overlays.

**Verdict: REJECT** as the whole answer. This is the right mechanism for the *environment page types*. It still treats the agent loop as primary and leaves GPU KV out of the object.

**Lesson:** Steal DeltaFS/DeltaCR (or Firecracker CoW) as the implementation of `FILE` and `PROC` pages.

### K11. “Unified snapshot of everything the agent is”

**Hypothesis:** One content-addressed image: model + KV + FS + effects + trace. `fork`/`merge`/`push`.

**Prior art:** **ProcessFork** (`manav8498/processfork`, PyPI `processfork`): exactly this five-layer `.pfimg`. Honest about maturity: generic CLI snapshots often have *empty* model/cache envelopes; live KV restore is adapter- and engine-version-specific; vLLM V1 is not bit-exact.

**Verdict: Closely explored → reject as the product.** ProcessFork is the existence proof that people already want a joint object. It is still *packaging an agent process*. It does not change the ABI: adapters wrap Claude Code, LangGraph, AutoGen. The loop remains.

**Lesson:** A bundle of layers is not an address space. An address space has one `fork`, one page table, and one mutation ISA. ProcessFork is five stores glued by a manifest.

### K12. “Speculative / parallel worlds for search”

**Hypothesis:** Fork environments, roll out in parallel, commit the winner.

**Prior art:** ParallelEnv (Tan, Zhang, Zaharia, 2026); Speculative Actions (arXiv:2510.04371); PASTE; Tree-of-Thoughts / MCTS agents; SWE-bench tree search.

**Verdict: REJECT** as a primitive. It is an application of cheap world forks. The primitive is the fork, not the search policy.

### K13. “Reversible traces / rewind”

**Hypothesis:** Time-travel debugging for cognition.

**Prior art:** Shepherd; AgentRewind (arXiv:2608.14380); “Revisable by Design” reversibility taxonomy (I/R/K/X); WebRollback; DART; rr/CRIU analogies.

**Verdict: Partially explored.** Exact replay of a stochastic model plus irreversible sinks is impossible. Weaker models (restore world + inject rewind memory; compensate K-class effects; refuse X-class without an outbox) are already being formalized.

**Lesson:** Reversibility is a property of *page types and effect classes*, not of “the agent.” Put it in the ISA.

### K14. “IFC / taint for agents”

**Hypothesis:** Information-flow control is the new security primitive.

**Prior art:** Fides (arXiv:2505.23643); APPA (arXiv:2607.24625); NeuroTaint; flowwarden; agent-sleuth; Arbiter-K taint on an instruction graph.

**Verdict: Closely explored** as security. Orthogonal and should be labels *on pages*, not a separate product.

### K15. “P2P opportunistic compute”

**Hypothesis:** `compute.request(GPU, latency, mem)` as an OS call.

**Prior art:** Prime Intellect protocol/marketplace; serverless GPU; Ray; disaggregated inference.

**Verdict: REJECT** as the core abstraction. Markets exist. They allocate machines, not worlds.

### K16. “Replicated worlds” (Croquet / TeaTime)

**Hypothesis:** Don’t send messages; replicate computation of a world against a timebase.

**Prior art:** Croquet Islands, TeaTime, modern Croquet reflectors. Requires deterministic operations. LLM forward passes are not deterministic in the TeaTime sense.

**Verdict: Adjacent, not a kill.** Croquet is the right *network idea* if LLM inference is moved *off-island* (an input event, like a keystroke), and only kernel-applied deltas are replicated.

**Lesson:** The protocol should replicate **committed world deltas**, not model tokens and not agent messages.

---

## 5. Convergence diagnosis

After the kill log, the field looks like this:

```text
                    cognition                         environment
                 ┌──────────────┐                 ┌──────────────────┐
   serving       │ KV, weights  │                 │ microVM, overlay │
                 │ thaw, ForkKV │                 │ DeltaBox, AENV   │
                 └──────┬───────┘                 └────────┬─────────┘
                        │                                  │
                        │     glued by manifests           │
                        │     (ProcessFork layers)         │
                        ▼                                  ▼
                 ┌─────────────────────────────────────────────┐
                 │              AGENT LOOP (still)             │
                 │   context → tool call → observation → …    │
                 └─────────────────────────────────────────────┘
                        ▲
                        │ validates / journals / taints
                 ┌──────┴───────┐
                 │ Arbiter-K,   │
                 │ Cordon,      │
                 │ Temporal     │
                 └──────────────┘
```

Every serious 2026 system improves one box. Almost none change the center.

Two papers come closest to moving the center:

1. **PatchBoard** — agents do not talk; they propose schema-valid JSON Patches; a kernel commits. The “world” is a JSON document.
2. **Arbiter-K** — the model is a PPU; a Semantic ISA is decoded and gated. The “program” is still an agent trajectory, and environment mutation is still `TOOL_CALL`.

The remaining gap is therefore precise:

> There is no **single schedulable object** whose pages *are* cognition, environment, authority, and log, whose **only** mutation interface is a typed delta, and whose **network/OS operations** (fork, migrate, replicate, attenuate) are defined on that object rather than on an agent wrapper.

That object is a **World Address Space**.

---

## 6. Surviving candidate

The rest of this document is the 16-part treatment of that candidate.

---

## 6.1 Concept

A **World** is a typed virtual address space:

```text
WorldId = hash(page_table_root)

PageType =
    FILE     -- overlay chunks, inodes, whiteouts
  | PROC     -- heap/FD/process tree (CRIU-class)
  | KV       -- model working-set blocks (paged attention)
  | CAP      -- unforgeable, attenuable capability nodes
  | LOG      -- append-only effect / provenance records
  | VIEW     -- materialized projection used as model input
  | LABEL    -- IFC lattice tags bound to other pages
```

A **World Kernel** schedules worlds, not agents. Language models are **devices** that may be attached to a world view. Attachment produces KV pages *inside* the same page table. The model never issues tool calls. It emits a **World Delta** in a small ISA. The kernel validates (capabilities, IFC, schema, reversibility class) and either:

- applies the delta on a copy-on-write fork and commits a new `WorldId`, or
- aborts, leaving the parent world unchanged.

An “agent” is a *scheduling policy* plus a set of attached devices. It is not a process type.

---

## 6.2 Fundamental insight

**Break A1–A5 together, not separately.**

Today’s systems keep the agent as process and the model as controller even when they add capabilities, journals, or snapshots. The insight is that those extras are properties of a **world page table**. Once the world is the process:

- Tool calls disappear (they were untyped mutations of an external heap).
- Memory as a separate database disappears (working set is `KV` and `FILE` pages).
- Agent migration becomes page mobility (some pages RDMA, some overlay, some capability tokens).
- Agent communication becomes attenuated world views, not chat.
- The loop is no longer fundamental: a world can progress because a timer, a human, a test harness, or a model attached.

This is the opposite of LLMOS (“LLM is the CPU”). Here the CPU of the world is a **deterministic kernel**. The LLM is closer to a GPU: a high-throughput, untrusted coprocessor that proposes numeric/symbolic updates the kernel may refuse.

---

## 6.3 Why the current assumption exists

It exists because it was the cheapest way to productize 2023-era APIs:

1. Chat completions already looked like a REPL.
2. Function calling gave vendors a JSON ABI without an OS.
3. Containers already isolated *something*, so “put the tools in a container” felt like isolation.
4. Context windows forced “memory” to be a bolt-on.
5. Heterogeneous environments (SaaS APIs, browsers, shells) have no common page type, so RPC looks inevitable.
6. GPU KV state is huge, vendor-locked, and was treated as a serving implementation detail, not process memory.

The 2026 explosion of Agent OS / Git-for-agents / transactional agents is the sound of that cheap architecture hitting long-running, concurrent, side-effecting workloads. The field is adding kernel words without changing the object of the kernel.

---

## 6.4 New abstraction

**World Address Space (WAS)** with operations:

```text
world_create(image) -> WorldId
world_fork(WorldId) -> WorldId                 -- CoW page table
world_view(WorldId, Caps) -> ViewId            -- attenuated projection
device_attach(WorldId, ModelSpec) -> DevId     -- may allocate KV pages
device_detach(DevId)
propose(DevId, ViewId) -> Delta                -- untrusted
validate(WorldId, Delta, Caps) -> Ok|Reject
commit(WorldId, Delta) -> WorldId              -- atomic, new root
abort(WorldId)                                 -- drop speculative fork
migrate(WorldId, Node, PageClass?)             -- partial residency
replicate(WorldId, Peers)                      -- committed deltas only
inspect(WorldId, PageRange)
rewind(WorldId, LogSeq) -> WorldId             -- if pages are R-class
```

There is no `spawn(agent)` as a primitive. You create a world, attach devices, and schedule `propose/commit`.

---

## 6.5 Architecture

### Components

```text
                          ┌─ Device: Model A (PPU)
                          ├─ Device: Model B
                          ├─ Device: Human
                          └─ Device: Test oracle
                                    │
                                    │  propose(Delta)
                                    ▼
┌──────────────────────────────────────────────────────────┐
│                     World Kernel                         │
│  scheduler │ validator │ IFC │ cap-space │ STM/MVCC      │
└──────────────────────────────────────────────────────────┘
                                    │
                                    │  commit new root
                                    ▼
┌──────────────────────────────────────────────────────────┐
│                 Content-addressed page store             │
│   FILE (overlay)  PROC (CRIU/template)  KV (paged)       │
│   CAP (ocap graph) LOG (WAL)  LABEL (lattice)            │
└──────────────────────────────────────────────────────────┘
                                    │
                    migrate / replicate / lazy page-in
                                    ▼
                         other machines / CXL / SSD
```

### Data structures

- **Page table:** Merkle tree. Root hash is `WorldId`. Each leaf is `(type, id, hash, labels, cap_rights)`.
- **Capability space:** object-capability graph. Handles are unforgeable. Attenuation is structural (InferNode/9P lesson: missing pages cannot be named).
- **Delta:** a transaction: list of ISA ops with read-set and write-set page ids (optimistic CC, as argued in the 2026 MAS concurrency-control position paper).
- **Log:** hash-chained. Irreversible ops are recorded as facts, never as “please retry” (ProcessFork’s effect-ledger / ACRFence lesson).
- **Device context:** which model id, sampling seed, and KV page range belong to an attachment. Detach may drop or persist KV pages.

### Execution model

Not ReAct. A kernel runqueue of worlds:

1. Pick a world that is runnable (pending input, attached device with budget, test failure, wall-clock).
2. Optionally `world_fork` for speculation.
3. Materialize `VIEW` pages (deterministic projection of readable pages; this *is* the “prompt,” but it is a kernel object).
4. `propose` from zero or more devices (parallel proposals are competing transactions).
5. `validate` + `commit` or `abort`.
6. Record LOG pages. Update scheduler.

A world with no attached model can still run: e.g. a compiled plan, a human, or a replay of deltas.

### State model

Immutable worlds. Mutation = new root. Identical to Git/Nix/ZFS in spirit, but pages are typed and some are GPU-resident.

### Communication

Not A2A tasks.

- **Grant:** send a capability to a `VIEW` (subset of pages).
- **Push:** send committed deltas (content-addressed; missing pages fetched like Unison/Git).
- **Reflector (Croquet-like):** for shared worlds, a sequencer timestamps deltas. Models are off-island: their `propose` events are inputs, like keystrokes. Only kernel-accepted deltas enter the replica.

### Failure handling

- Crash: last committed `WorldId` + LOG is durable. Resume by mapping pages (lazy).
- Conflict: optimistic validation fails → abort speculative fork; optionally re-propose.
- Device crash: KV pages may be lost; world remains; re-attach recomputes or restores KV from store (thaw/ProcessFork adapters).
- Irreversible sink: never in the speculative path. Cordon-style **outbox**: X-class ops become LOG facts only after a commit barrier.

### Scheduling

Schedule **worlds** and **devices** separately:

- Worlds: fairness, human priority, test-queue, speculative-search budget (ParallelEnv policy sits here).
- Devices: GPU time, token budget. Preempt by detaching (KV pages may stay in the world or spill to SSD).

This is closer to a database + device scheduler than to “agent time slices.”

### Storage

Single content-addressed store. Dedup across forks (ProcessFork’s 1.004× storage for 12 FS forks is the existence proof for FILE pages; KV pages need the same CoW). Spill: FILE on object storage (AgentFS/Turso direction), KV on RDMA/SSD (ObjectCache/Mooncake direction), PROC as incremental CRIU images.

### Security

- **No ambient authority.** Device code (the model) cannot name pages outside the view.
- **Capabilities** on commit, not on “who the agent is.”
- **IFC labels** on pages; tainted VIEW pages cannot flow into privileged FILE/network ops without a `VERIFY` / endorsement page (Arbiter-K/Fides/APPA).
- **Reversibility class** on each ISA op (I/R/K/X). Kernel refuses X-class outside an outbox transaction.

### Hardware interaction

The kernel *does* understand topology, because pages have homes:

- `KV` pages prefer GPU HBM, may live in CPU/CXL/SSD.
- `FILE` pages prefer NVMe; hot chunks can be page-cached.
- `PROC` pages are host-ISA specific; migration may require respawn (ProcessFork’s two-tier CRIU vs argv/cwd respawn).
- A world may be **partially resident**: KV on a datacenter GPU, FILE on a laptop, CAP on a phone’s secure element.

Partial residency is the migration story that 1990s mobile agents lacked: you do not move “the agent.” You move the pages that are cold or that the next `propose` needs.

---

## 6.6 Execution walkthrough

Task: “Fix the failing test in this repo, without emailing anyone.”

```text
world_create(repo_image)                -> W0
  FILE pages = overlay of the git worktree
  CAP pages  = {read/write /workspace, run tests, no net}
  LOG empty

device_attach(W0, local_llm)            -> D
  allocates KV pages in W0 (empty)

loop:
  VIEW = project(W0, caps)              -- kernel, deterministic
  delta = propose(D, VIEW)              -- untrusted
  match validate(W0, delta):
    Reject(no_net) -> log, continue
    Ok ->
      W1 = fork(W0)
      apply(W1, delta)                  -- e.g. WRITE_FILE, EXEC pytest
      if tests_green(W1): commit W1; halt
      else: W0 = W1                     -- keep going, or
            fork extra speculative W2   -- ParallelEnv policy
```

Checkpoint is free: `W0` still exists. Migration mid-task: spill FILE+LOG to object storage, move KV pages to another GPU, `device_attach` there, continue from the same `WorldId`. No context reconstruction speech about “here is a summary of what we did”; the pages *are* what we did.

There is still a loop in the prototype. It is a **kernel scheduler loop**, not an agent loop. The model does not choose to “call pytest”; it proposes `EXEC` against a capability it holds. The distinction is the ABI and the object, not the presence of iteration.

---

## 6.7 Why current systems don’t already do this

| System | Why it is not a World Address Space |
|---|---|
| LangGraph / Agents SDK / AutoGen / CrewAI | Checkpoint Python dicts; tools escape the state object |
| OpenHands / SWE-agent / browser-use | Environment is a sidecar; cognition is messages |
| E2B / Daytona / AgentENV / Firecracker | Fast *environment* fork; no KV/cap/log in the same table |
| DeltaBox | Best-in-class FILE+PROC C/R; agent loop still owns control; no KV pages |
| thaw / ForkKV | Best-in-class KV CoW; no world |
| ProcessFork | Five-layer *manifest* around existing agents; empty engine layers on the generic path; still adapters over the loop |
| Temporal / Restate | Journals the interpreter |
| MCP / A2A | RPC |
| InferNode / 9P | Best namespace story; LLM is still a file you write prompts to; no unified page table with KV |
| PatchBoard | Kernel-committed mutations, but JSON schema not an address space |
| Arbiter-K | PPU + ISA + taint, but ISA still has `TOOL_CALL` and the program is an agent trajectory |
| Shepherd | Reversible traces of tasks; not a page table |
| Cordon | Semantic transactions over tools |
| Croquet | Replicated worlds for deterministic objects; no LLM-as-device, no KV pages |
| Unison / Nix | Content-addressed *code* and builds, not live KV+env worlds |
| Ray / K8s / microVMs | Place containers, not typed pages of a world |

The comparison that matters is **ProcessFork + Arbiter-K + DeltaBox + InferNode**. A strong engineer could glue them. The claim is that the glue *is* the missing ABI: one page table, one `fork`, one delta ISA, devices instead of agents. If that glue is only an adapter, the idea dies (see kill criteria).

---

## 6.8 Prior-art investigation

### Papers / systems (closest first)

| Work | Relation | Difference |
|---|---|---|
| Arbiter-K, arXiv:2604.18652 | LLM as PPU; Semantic ISA; kernel gates sinks | Trajectory ISA with `TOOL_CALL`; no world page table |
| PatchBoard, arXiv:2605.29313 | Propose/validate/commit mutations | JSON document, not pages; no KV |
| ProcessFork | Joint image of model, KV, world, effects, trace | Manifest of layers; wraps existing loops |
| DeltaBox, arXiv:2605.22781 | DeltaState OS abstraction for FILE+PROC | No cognition pages; search-oriented C/R |
| thaw | Git operations on live KV | KV only |
| ForkKV, arXiv:2604.06370 | `fork`+CoW for KV | KV only; serving, not worlds |
| InferNode / 9P namespace agents | Structural capability via missing paths | Files as API; classical agent still sits on top |
| Agent libOS, arXiv:2606.03895 | AgentProcess + caps + checkpoints | Tools remain the ABI; agent is the process |
| Cordon, arXiv:2606.17573 | Shadow state + outbox | Tool transactions |
| Shepherd, arXiv:2605.10913 | Reversible effect traces | Task/trace graph |
| Fides / APPA | IFC for agents | Labels, not address spaces |
| Croquet / TeaTime | Replicated worlds, off-island I/O | Deterministic objects; human collaboration |
| Unison | Content-addressed moving computations | No GPU KV, no OS environment pages |
| KeyKOS / EROS / seL4 | Persistent capability processes | No model working set as pages |
| Telescript / Aglets | Mobile agents | Process mobility without a typed world |
| MemGPT / Letta | Virtual memory for *context* | Context only |
| ParallelEnv | Isolated env snapshots for search | Application of forks |
| AgentFS / Factory vfs | Queryable SQLite agent filesystem | FILE plane only |
| Prime Intellect | P2P compute | Machine allocation |

### Patents / commercial

No production system was found that ships a joint KV+environment page table as an OS ABI. ProcessFork and thaw are the commercial/OSS attempts at *images*. E2B/Daytona/AgentENV ship environment snapshots. None advertise models as devices on a world.

Search coverage included arXiv, GitHub, PyPI, vendor blogs, and terminology expansions: agent migration, mobile agents, KV migration, sandbox checkpoint, semantic ISA, world snapshot, content-addressed agent state, DeltaState, processfork, replicated computation.

---

## 6.9 Novelty assessment

**Classification: partially explored → investigate further; as a unified ABI, mostly unexplored.**

Evidence:

- Every *component* has a 2025–2026 citation (KV CoW, overlay C/R, ocap/9P, PPU+ISA, semantic transactions, replicated worlds).
- No system found defines **one page table** spanning those components and **eliminates tool calling** in favor of world deltas.
- ProcessFork is the contradiction search’s strongest hit. It is an image format and adapters, not a kernel ABI, and its own docs say model/cache layers are placeholders on the generic path.
- Arbiter-K is the strongest conceptual hit on “LLM as untrusted proposer.” It retains tools and agent trajectories.

Language used carefully: **I found no direct implementation of a World Address Space kernel after searching the names above.** The architectural distinction is the object of scheduling (world vs agent) and the mutation ISA (typed page updates vs tool RPC). Novelty remains uncertain because a lab could have an unindexed prototype, and because composition of ProcessFork+Arbiter-K could be argued to be “close enough.”

---

## 6.10 Technical challenges

| Challenge | Kind |
|---|---|
| Bit-exact KV restore across engines and CUDA graphs | Fundamentally hard (ProcessFork already failed this on vLLM V1) |
| Irreversible external APIs (email, wire transfer) | Fundamentally hard; only outbox + human/endorsement |
| Semantic merge of two worlds after true divergence | Fundamentally hard; do not pretend Git-merge of cognition |
| Heterogeneous PROC pages (different ISAs, browsers) | Fundamentally hard for strong mobility; weak mobility (respawn) is the honest default |
| Defining a small, stable World ISA that covers shell, FS, browser, GPU | Engineering hard; the ISA will churn |
| Atomic commit across GPU memory and overlayfs | Engineering hard; need a quiesce protocol (DeltaBox’s LLM-wait trick) |
| IFC through a neural proposer | Fundamentally hard; conservative taint on whole VIEWs is the only sound start (Fides) |
| Multi-tenant GPU + CoW page tables | Engineering hard (ForkKV is the template) |
| Partial residency / lazy page-in over WAN | Engineering hard (Firecracker uffd, ObjectCache) |
| “No agent loop” in the first prototype | Merely inconvenient; a scheduler loop is enough |
| Convincing models to emit ISA instead of tool JSON | Merely inconvenient; constrained decoding / grammar |

Exact replay of a live agent, including vendor APIs and stochastic decoding, is **currently impossible**. The consistency model is: **committed worlds are bit-identical in FILE/CAP/LOG; KV is best-effort / engine-specific; X-class effects are facts, not rewind points.**

---

## 6.11 Prototype (one strong engineer)

**Goal:** falsify or support the claim that a *single* CoW page table with a tiny delta ISA can replace tool-calling for a SWE-like task, with fork cheaper than process clone and with the model unaware of “tools.”

**Language / runtime:** Rust kernel in user space. Linux only for v0.

**Hardware:** laptop CPU is enough for the ISA/FS path. Optional one GPU for a real `KV` page type later.

**Dependencies:** overlayfs or a user-space CoW tree (start with a content-addressed blob dir). `bubblewrap` or a Firecracker microVM for `EXEC`. A local model via llama.cpp *or* a mocked proposer that emits ISA JSON.

**Must be real:**

- Merkle page table with `FILE` + `LOG` + `CAP`
- `fork` as copy-on-write (don’t copy bytes)
- ISA: `READ`, `WRITE`, `EDIT`, `EXEC` (jailed), `GRANT`, `REVOKE`
- Validator: cap check + simple IFC (secret files cannot be `EXEC`ed to curl)
- Scheduler loop: view → propose → validate → commit
- Persist `WorldId` to disk; resume after process death without “summarize the conversation”

**May be mocked:**

- `KV` pages (treat context as a FILE page of tokens in v0)
- `PROC` CRIU (kill/respawn the jail)
- Multi-device speculation
- Network replication
- Browser DOM pages

**Must not do:** wrap LangGraph, speak MCP, or implement A2A. If the prototype grows a `tools` array, it has failed.

**Size:** a few thousand lines plus tests. Not an OS. A library + `was` CLI: `was create`, `was fork`, `was propose`, `was checkout <world-id>`.

---

## 6.12 Research question

> Can autonomous execution be made portable, forkable, and isolatable by representing the unit of scheduling as a typed, content-addressed **world address space**, with language models attached only as untrusted **proposal devices** over attenuated views, rather than as controllers that emit tool calls against an external environment?

A sharper systems question:

> Does a single copy-on-write page table spanning filesystem, capabilities, effect log, and (later) KV blocks provide fork/migrate/rollback cost and safety properties that layering ProcessFork-style images on a ReAct loop cannot?

---

## 6.13 Evaluation

**Baseline:** SWE-agent or OpenHands-style loop in the same jail, same model, same repo. Second baseline: ProcessFork-style snapshot of that loop (FS+trace only, matching what actually ships).

**Workloads:** a slice of SWE-bench-verified (or a 20-task internal fixture if GPU/time is limited); a destructive-tool suite (rm, curl exfil, git push); a fork-fanout suite (N speculative patches, pick tests-green).

**Metrics:**

- Task success (same as baseline)
- Irreversible-effect incidents (should be ~0 with outbox)
- Fork latency and extra bytes vs N (target: fork ≪ clone, storage sublinear in N, as in DeltaBox/ProcessFork)
- Resume-after-kill: time-to-next-action without re-reading the repo via the model
- Prompt-injection / exfil ASR on a small AgentDojo-like set (namespace + IFC)
- KV (v1): restore quality (output-equivalent vs bit-exact; expect the latter to fail)

**Hardware:** CPU jail for v0; one GPU box for KV v1.

**Expected failure modes:**

- Model refuses to speak ISA / wastes tokens; constrained decoding needed.
- `EXEC` is just a tool with extra steps (this is a **kill** if true; see below).
- Quiesce for commit is as slow as naive CRIU.
- Views dump too many files into the prompt ( InferNode’s token-reduction claim should be re-measured).

---

## 6.14 Potential impact

If the hypothesis holds, the new category is not “an agent framework.” It is:

- a **runtime primitive** (`WorldId`, `fork`, `commit`) analogous to `pid` and `fork`
- an **OS abstraction** (typed page table for autonomous work)
- a **network protocol** (capability-attenuated world views + sequenced deltas, not agent chat)
- a **storage system** (content-addressed typed pages, including GPU working sets)
- a **device model** for accelerators (models as proposal devices)

That is closer to “processes and files for a world that includes neural working memory” than to another orchestration layer.

Downstream applications (coding agents, computer use, multi-agent) would be policies on worlds. They are out of scope for the primitive.

---

## 6.15 Kill criteria

Abandon the idea if any of these hold:

1. **Prior-art kill:** a system is found that already exposes one page table + delta ISA + device-attached models (not a five-layer image adapter). ProcessFork in its current form does *not* meet this; a future ProcessFork kernel might.
2. **Composition kill:** implementing the prototype as a thin adapter over DeltaBox + constrained tool calling + overlayfs yields the same metrics. Then there is no new primitive, only packaging.
3. **ABI collapse:** every useful ISA op is 1:1 with a tool schema, and models/validators gain nothing from page identity, CoW, or capabilities-as-missing-pages.
4. **KV impossibility:** if even output-equivalent restore of KV pages cannot be bound to `WorldId` without engine-private hooks that break on every vendor release, drop `KV` as a page type and re-evaluate whether FILE+CAP+LOG alone is still distinct from AgentFS+Cordon. If it is not, kill the whole candidate.
5. **Irreversibility dominance:** if real tasks are mostly X-class external APIs, world forks do not buy rollback, and the system collapses to an outbox workflow engine (already invented).

---

## 6.16 Next experiment (smallest falsifier)

**Do not build the kernel first.**

Build a **two-world race** on one repo:

1. Freeze a worktree as content-addressed FILE pages (`WorldId` = Merkle root).
2. `fork` twice (CoW).
3. In each fork, apply a *scripted* delta ISA log (no model): edit file, run tests.
4. Measure fork+apply vs `cp -a` / container clone.
5. Kill the process, resume from `WorldId`, confirm the worktree and log match.

**If CoW fork is not materially cheaper and resume is not bit-identical for FILE+LOG, stop.** The later ML parts cannot save a bad state object.

**If it works,** add a constrained-decoding proposer (small local model) that may only emit ISA ops, with capabilities that omit network. Compare exfil ASR and task success against the same model with ordinary tools. **If ASR and success are indistinguishable,** the ABI is a costume; kill it.

Only then attach real KV pages.

---

## 7. Runner-up that was demoted

**Causal continuation objects** (delimited continuations + reversible effects as the process). Shepherd, Etas, and durable-execution replay already occupy this. Continuations are an implementation technique for `propose` (the rest of the kernel after a device yield). They are not the object.

**Single-level store / KeyKOS persistence for agents** is historically deep and should inform the page store, but without typed KV/FILE/CAP pages it becomes “orthogonal persistence of a Python agent,” which is CRIU.

---

## 8. What this is not

Not LangGraph. Not an Agent OS wallpaper. Not MCP. Not “Git for chats.” Not a dashboard. Not computer-use. Not a coding agent. Those can *use* worlds. They must not define them.

---

## 9. Open questions worth a second paper, not this primitive

- Hardware page tables that actually span HBM and NVMe (CXL). Fascinating, not required to test the ABI.
- Browser as a world page type (DOM as FILE-like). Important, after shell+FS.
- Multi-model commit (two PPUs proposing on one world) as a concurrency-control problem — the MAS serializability papers already frame it.
- Economic compute markets: orthogonal.

---

## 10. References (search anchors)

Not a complete bibliography; these are the works that killed or shaped hypotheses.

- Mei et al., *AIOS: LLM Agent Operating System*, arXiv:2403.16971
- *Agent Operating Systems (AOS)*, arXiv:2606.01508
- *Agent libOS*, arXiv:2606.03895
- Wen et al., *Arbiter-K / Semantic ISA*, arXiv:2604.18652
- *PatchBoard*, arXiv:2605.29313
- Dong et al., *DeltaBox / DeltaState*, arXiv:2605.22781
- *Cordon*, arXiv:2606.17573
- *Agentic Transaction*, arXiv:2608.13900
- *MAS Should Prioritize Concurrency Control*, arXiv:2608.18092
- *Shepherd*, arXiv:2605.10913
- *AgentRewind*, arXiv:2608.14380
- *Speculative Actions*, arXiv:2510.04371
- *ForkKV*, arXiv:2604.06370
- Fides, arXiv:2505.23643; APPA, arXiv:2607.24625
- AgentGit, arXiv:2511.00628
- Etas, arXiv:2607.17780
- thaw, https://thaw.sh/ ; https://github.com/thaw-ai/thaw
- ProcessFork, https://github.com/manav8498/processfork
- InferNode, https://infernode.io/
- AgentENV, https://github.com/kvcache-ai/agentenv
- Factory AgentFS, https://github.com/Factory-AI/vfs
- Croquet / TeaTime (Smith et al., 2003)
- Unison language, content-addressed computation
- Telescript; IBM Aglets
- Google A2A; Anthropic MCP
- Prime Intellect protocol

---

## 11. Conclusion

The agent stack has converged on a conversation-shaped interpreter with tools as RPC. The 2026 systems literature is wrapping that interpreter in OS, Git, transaction, and sandbox machinery. Those wrappers are valuable and should be reused as **page implementations**.

The abstraction that still appears under-built is the **world as the process**: a typed address space in which model working memory, files, capabilities, and effects are pages; models are devices; mutation is a validated delta; fork/migrate/replicate are page-table operations.

That is a research hypothesis, not a claim of invention. The smallest experiment in §6.16 can kill it in a week. If it survives, the follow-on is a kernel ABI, not another agent framework.
