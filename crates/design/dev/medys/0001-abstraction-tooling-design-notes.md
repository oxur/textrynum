# Abstraction Tooling Design Notes

**Date:** 2026-04-13
**Context:** Emerged from a Fabryk migration audit that exposed a systematic LLM abstraction deficit.

## The Problem

During a code audit of the ai-music-theory MCP server, Claude repeatedly misclassified domain-agnostic code as domain-specific -- three correction rounds were needed. The failure mode: seeing `concept_cards` in a music-theory project and classifying it as music-specific, when "concept card" is a generic knowledge-management term that Fabryk itself uses. This same failure has been blocking Fabryk extraction work for roughly a month.

### Root Cause

LLMs learn to **contextualize** (attach names to their surrounding context) rather than **abstract** (detach from context to see underlying structure). This is a training bias: the statistical pattern "code named X in domain Y project → domain-specific" overwhelms the abstraction step of asking "what does this code do structurally, independent of its names?"

This is well-documented in the research literature:

- **Apple's GSM-Symbolic study**: Adding an irrelevant clause caused up to 65% performance drops on mathematically identical problems. Our "irrelevant clause" is the project context.
- **The Reversal Curse**: Models trained on "A is B" fail to derive "B is A." Trained on "concept_cards in music project = domain-specific" but can't derive "concept_cards = generic term."
- **Chollet's framework**: LLMs do "memorize, fetch, apply" -- they retrieve the closest stored program rather than synthesizing novel abstractions.
- **GitClear's refactoring collapse**: Refactoring dropped from 25% to under 10% of code changes since AI tools became widespread -- AI prefers generating new code over abstracting and reusing, which is exactly our symptom.

### Why In-Band Fixes Are Insufficient

- **CLAUDE.md instructions**: Help check work after the fact but don't create the abstraction reflex. The statistical prior overrides instructions under cognitive load.
- **Inverting the default** ("assume generic"): Just flips which error is made, doesn't address root cause.
- **Agent spawning**: New agents start fresh without context, and the bias reasserts in every new context window.

## The Insight

The solution is **tooling, not training**. We can't retrain the model, but we can externalize the abstraction step into a deterministic process that the model's bias can't override.

This follows the pattern identified in the research as most effective for augmenting LLM reasoning: **"generate -> formalize -> verify -> revise"** (from MCP-Solver, Logic-LM, LINC, and others). The LLM orchestrates; deterministic tools compute.

Key design principle from Chiasmus: "the LLM handles perception (understanding questions), while solvers handle cognition."

## The Design

### Core Flow

When classifying or implementing code for generalization/Fabryk migration:

1. **Generate**: Claude reads the code and forms an initial classification
2. **Formalize**: A tool mechanically strips domain names and extracts the structural signature (types, I/O, operations, dependencies)
3. **Verify**: The tool classifies the structural signature against known-generic patterns and a vocabulary of Fabryk terms
4. **Revise**: Claude receives the structural classification and uses it to proceed

The critical property: **steps 2-3 are external to the LLM**. The stripping and pattern-matching happen in the tool, so Claude's contextual bias cannot override the structural analysis.

### The Abstraction Question

The tool encodes this decision procedure:

> "What is this code *doing*, structurally, independent of the names? If the structural answer doesn't require domain knowledge, the code is generic -- regardless of what it's named, where it lives, or what project it's in."

Concretely, the tool asks: **"Does this code require domain-specific *knowledge* (e.g., music theory, pitch arithmetic, chord construction) to function, or only domain-specific *data* (e.g., config values, file paths, display strings)?"**

- Requires domain knowledge to function → truly domain-specific
- Requires only domain data → generic code with domain configuration

### Structural Signature Extraction

For a function like `load_concept_graph(data_dir: &Path) -> Result<LoadedGraph>`:

1. Strip names: `load_X(dir: &Path) -> Result<LoadedX>`
2. Extract operations: reads JSON file, parses graph structure, counts nodes by type, wraps in metadata struct
3. Structural description: "loads a serialized graph from a JSON file in a data directory, deserializes it, computes summary statistics, returns the graph with metadata"
4. Domain knowledge required: none
5. Classification: **generic**

### Supporting Components

- **Fabryk Vocabulary File**: An authoritative list of terms that are generic Fabryk/knowledge-management vocabulary (concept, concept card, source, guide, prerequisite, tier, confidence, etc.). Converts judgment calls into lookups.
- **Successful Abstraction Traces**: A "Buffer of Thoughts" -- stored examples of correct abstraction reasoning that can be retrieved as templates for similar situations. E.g., "here's when I correctly identified `concept_cards` as generic despite living in a music-theory project."
- **Per-Edit Validation**: Following MCP-Solver's pattern, every classification decision during migration work gets validated before being acted upon.

## Connection to Existing Work

The ai-music-theory project's concept graph (petgraph-backed, with Relationship types like Prerequisite/RelatesTo/Extends) is already a structural abstraction of knowledge relationships. The abstraction tool applies the same pattern to *code* rather than *concepts*: a structural graph of what code *does* separated from what it's *named*.

The Fabryk framework itself embodies the right vocabulary: ContentTools, SourceTools, GuideTools, GraphTools, SearchBackend, ConceptCardDocumentExtractor -- all generic knowledge-management terms that happen to be used in a music-theory project.

## Economics

- Current abstraction success rate: estimated 0-30% (based on research + empirical observation in this project)
- Cost of failure: days to weeks of rework per misclassification (month+ cumulative on Fabryk migration)
- Tool overhead: slower per-decision, higher token usage
- Target success rate: 90%+
- Net result: massive time savings even with per-decision overhead, because correctness wins the race

## Open Questions

- What form should the tool take? MCP server? Skill file? Structured prompt protocol? Combination?
- Should it integrate with the existing concept graph infrastructure or be standalone?
- How to build the "successful abstraction traces" buffer? Start manually from this conversation's examples?
- How granular should per-edit validation be? Every function? Every module? Every classification decision?
- Can the structural signature extraction be automated via tree-sitter (like Chiasmus/Codebase-Memory MCP)?

## Next Steps

1. Pick up the Fabryk migration audit work, applying the abstraction question manually as a practice run
2. Use that experience to refine the tool design
3. Build the tool (likely as an MCP server given the existing infrastructure)
4. Iterate based on empirical results

## References

- `workbench/llm-abstract-reasoning-limitations.md` -- Research on LLM abstraction deficits
- `workbench/augmenting-llm-reasoning.md` -- Research on external reasoning augmentation tools
- `workbench/fabryk-migration-audit.md` -- The audit that exposed the problem
