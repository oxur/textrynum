# Update Architecture Diagram

## Prerequisites

Load the Dendryform YAML schema. If `assets/ai/DENDRYFORM-YAML-SCHEMA.md` exists, read it. Otherwise, download it using `curl` via the Bash tool:

```bash
curl -sL https://raw.githubusercontent.com/oxur/dendryform/refs/heads/main/assets/schema/DIAGRAM-YAML-SCHEMA.md \
  -o assets/ai/DENDRYFORM-YAML-SCHEMA.md
```

Do not use WebFetch for this — it summarizes content instead of returning it verbatim.

The downloaded file is `.gitignore`d and serves as a local cache. Re-download if stale.

Read the schema fully before proceeding. All YAML output must conform to it.

## Step 1: Examine the Codebase

Perform a thorough examination of the project source code in the current working directory. Build an internal model of the software architecture covering:

- **Crate/package/module hierarchy** — all top-level units, their names, purposes, and public API surface
- **Dependency relationships** — which units depend on which, including feature-gated and optional dependencies
- **Layering** — identify architectural tiers (e.g., interface, domain, infrastructure, external)
- **Extension points** — traits, interfaces, plugin systems, registries
- **External dependencies** — significant third-party libraries and what they provide
- **Consumers** — who/what uses this system (applications, users, other services)
- **Key abstractions** — the important types, traits, and patterns that define the architecture

Read `Cargo.toml` (or equivalent manifest) files, `src/lib.rs` and `src/main.rs` entry points, and public module declarations. Follow dependency chains through the workspace. Do not guess — read the actual source.

## Step 2: Compare with Existing Architecture

Read `scripts/update-arch.sh` to find the path to the architecture YAML file. Read that YAML file.

Compare your model from Step 1 against the existing YAML:

- **New crates/modules** not represented
- **Removed crates/modules** still present in the YAML
- **Changed relationships** (new dependencies, removed dependencies, renamed items)
- **Outdated descriptions** (stale tech lists, wrong trait names, incorrect counts)
- **Structural changes** (layers reorganized, containers added/removed)

If the existing YAML already accurately represents the current architecture, report "No changes needed" and stop.

## Step 3: Update the Architecture YAML

Write an updated YAML file to the same path. Preserve the overall style and conventions of the existing file (theme, color scheme, icon choices, legend structure) while making it accurate.

Guidelines:

- Every crate/package/module in the project should be represented as a node
- Every real dependency relationship should be represented as an edge
- Descriptions should be specific and accurate — mention actual trait names, tool counts, algorithms, etc.
- Tech tags should list the key libraries or features, not generic terms
- Feature-gated relationships should note the feature name in the edge label
- Use containers to group related nodes into logical subsystems
- Use connectors between tiers to show the flow direction
- The legend should cover all colors used in the diagram

## Step 4: Generate the Diagram

Run the update script:

```bash
scripts/update-arch.sh
```

If `dendryform` is not found, instruct the user to install it:

```bash
cargo install dendryform
```
