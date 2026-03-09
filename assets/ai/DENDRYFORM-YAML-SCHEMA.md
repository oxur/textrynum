# dendryform YAML Schema Reference

> **Version:** 1.0 — **For:** AI assistants generating architecture diagrams
>
> This is the complete reference for authoring `dendryform` YAML files.
> A valid YAML file produces HTML, SVG, and PNG architecture diagrams.

---

## Document Structure

```yaml
diagram:       # Required. Metadata and theming.
layers:        # Required. Ordered visual elements (tiers, connectors, flow labels).
legend:        # Optional. Color key rendered at the bottom.
edges:         # Optional. Semantic relationships (for Mermaid/Structurizr export).
```

---

## `diagram`

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `title.text` | string | yes | Title text (appears after accent word) |
| `title.accent` | string | yes | Word rendered in accent color (green) |
| `subtitle` | string | yes | Subtitle in monospace below the title |
| `theme` | string | yes | `"dark"` (built-in) or path to theme YAML |

**Renders:** Centered header block. Title displays as `accent · text`.

---

## `layers`

An ordered list. Each item is **exactly one of** these types:

### `tier`

A horizontal band of nodes, optionally wrapped in a container.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `id` | slug | yes | Unique identifier (lowercase, hyphens, dots) |
| `label` | string | no | Tier heading (uppercase monospace) |
| `layout` | layout | no | How nodes are arranged (see below) |
| `nodes` | list | no | Nodes in this tier (omit if tier has only a container) |
| `container` | container | no | Wraps content in a bordered box with a floating label |

**Layout values:**

| Value | Renders as |
|-------|------------|
| `single` | Full-width, centered node |
| `grid: { columns: N }` | CSS grid with N equal columns |
| *(omitted)* | Defaults to `auto` (1 column per node, up to 4) |

**Renders:** `.tier` div with optional `.tier-label`, then nodes in the specified grid.
A `single` tier centers the node and adds extra padding.

### `connector`

A visual link between adjacent tiers.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `style` | string | yes | `"line"` (solid with arrowhead) or `"dots"` (5 dots) |
| `label` | string | no | Protocol/description label (line connectors only) |

**Renders:**
- `line` → vertical 2px line with downward arrow, label floated to the right of center
- `dots` → 5 small dots in a horizontal row, centered

### `flow_labels`

Directional labels between tiers (typically above external services).

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `items` | list of strings | yes | Labels, each rendered with a `↓` arrow prefix |

**Renders:** Horizontal row of `↓ label` items, evenly spaced.

---

## `container`

A bordered box with a floating label. Used for server boundaries,
subsystem groupings, etc. Appears inside a `tier`.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `label` | string | yes | Floating label text (e.g., `"taproot server · cloud run"`) |
| `border` | string | yes | `"solid"` or `"dashed"` |
| `label_color` | color | yes | Color of the floating label text |
| `layers` | list | yes | Nested layers (tiers, connectors) inside the container |

**Renders:**
- `solid` → rounded border, opaque background, label floated top-left
- `dashed` → dashed border, tinted background matching `label_color`, smaller label

Containers nest recursively. A tier inside a container can itself have a container.

---

## `node`

An individual card within a tier.

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| `id` | slug | yes | — | Unique across entire diagram |
| `kind` | kind | yes | — | Semantic type (for export) |
| `color` | color | yes | — | Accent color for top-bar, icon, hover |
| `icon` | string | yes | — | Single Unicode character |
| `title` | string | yes | — | Bold heading |
| `description` | string | yes | — | Body text |
| `tech` | list of strings | no | `[]` | Technology badges |

**Node `kind` values:** `person`, `system`, `container`, `component`, `infrastructure`, `group`

**Renders:** Card with 2px colored top-bar, icon + title row, description, tech badge pills.
Hover lifts the card and highlights the border in the node's color.

---

## `color`

One of: `blue`, `green`, `amber`, `purple`, `red`, `teal`

Each color provides: accent (top-bar, icon), dim (tinted background), hover border.

---

## `legend`

List of color/label pairs.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `color` | color | yes | Swatch color |
| `label` | string | yes | Legend text |

**Renders:** Horizontal row of `[■] Label` items, centered at the bottom.

---

## `edges`

Semantic relationships between nodes. Not rendered visually in HTML
(use connectors and flow_labels for visual connections). Used by
Structurizr and Mermaid exporters.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `from` | slug | yes | Source node ID |
| `to` | slug | yes | Target node ID |
| `kind` | edge_kind | yes | `uses`, `reads`, `writes`, `deploys`, `contains` |
| `label` | string | no | Relationship description |

---

## Validation Rules

1. All node `id` values must be unique across the entire diagram
2. All edge `from`/`to` must reference existing node IDs
3. Node IDs must be valid slugs: `[a-z0-9][a-z0-9._-]*`
4. No empty tiers (a tier must have `nodes` or a `container`)
5. Container nesting depth max: 3 levels

---

## Annotated Minimal Example

```yaml
diagram:
  title:
    text: "system architecture"
    accent: "myapp"
  subtitle: "a web application with a database"
  theme: dark

layers:
  # Full-width centered node
  - tier:
      id: clients
      label: "Clients"
      layout: single
      nodes:
        - id: browser
          kind: person
          color: blue
          icon: "◇"
          title: "Web Browser"
          description: "End users access the app via HTTPS"
          tech: ["React", "TypeScript"]

  # Line connector with protocol label
  - connector:
      style: line
      label: "HTTPS"

  # Server container with nested tiers
  - tier:
      id: server
      container:
        label: "application server · AWS"
        border: solid
        label_color: green
        layers:
          - tier:
              id: api
              label: "API Layer"
              layout:
                grid:
                  columns: 2
              nodes:
                - id: rest-api
                  kind: component
                  color: green
                  icon: "◈"
                  title: "REST API"
                  description: "Request routing and validation"
                  tech: ["axum"]
                - id: auth
                  kind: component
                  color: green
                  icon: "◈"
                  title: "Auth"
                  description: "JWT verification"
                  tech: ["jsonwebtoken"]

          # Dot connector between sub-tiers
          - connector:
              style: dots

          # Nested container (dashed border)
          - tier:
              id: core
              label: "Core"
              container:
                label: "business logic"
                border: dashed
                label_color: purple
                layers:
                  - tier:
                      id: services
                      layout:
                        grid:
                          columns: 2
                      nodes:
                        - id: users
                          kind: component
                          color: purple
                          icon: "◆"
                          title: "User Service"
                          description: "Account management"
                        - id: orders
                          kind: component
                          color: purple
                          icon: "◆"
                          title: "Order Service"
                          description: "Order processing"

  # Flow labels above external services
  - flow_labels:
      items:
        - "SQL queries"
        - "cache reads"

  # External services tier
  - tier:
      id: external
      label: "External Services"
      layout:
        grid:
          columns: 2
      nodes:
        - id: postgres
          kind: infrastructure
          color: red
          icon: "◯"
          title: "PostgreSQL"
          description: "Primary data store"
          tech: ["RDS"]
        - id: redis
          kind: infrastructure
          color: teal
          icon: "◯"
          title: "Redis"
          description: "Session cache"
          tech: ["ElastiCache"]

legend:
  - color: blue
    label: "Clients"
  - color: green
    label: "API Layer"
  - color: purple
    label: "Business Logic"
  - color: red
    label: "Database"
  - color: teal
    label: "Cache"

edges:
  - from: browser
    to: rest-api
    kind: uses
    label: "HTTP requests"
  - from: rest-api
    to: auth
    kind: uses
    label: "token validation"
  - from: rest-api
    to: users
    kind: uses
  - from: rest-api
    to: orders
    kind: uses
  - from: users
    to: postgres
    kind: reads
  - from: orders
    to: postgres
    kind: reads
  - from: orders
    to: redis
    kind: reads
    label: "session lookup"
```

---

## Icon Conventions

| Use for | Icon | Example |
|---------|------|---------|
| External clients/users | ◇ | Browser, mobile app |
| Server-side components | ◈ | API, auth, config |
| Tool/capability groups | ⊕ | Tool registries |
| Engine/subsystem internals | ◆ | Search, graph, cache |
| Client libraries | ◉ | DB client, HTTP client |
| External services | ◯ | Database, SaaS, cloud |

These are conventions, not enforced. Any single Unicode character works.

---

## Common Patterns

**Single external client → server → external services:**
Use `single` layout for client, `solid` container for server, `grid` for services.

**Subsystem grouping (e.g., knowledge engine):**
Use `dashed` container nested inside the server container.

**Multiple grid densities in one container:**
Use multiple sub-tiers with different `grid.columns` values.

**Visual flow between tiers:**
Use `line` connector (with label) for primary connections,
`dots` connector for internal flow within a container,
`flow_labels` for labeled arrows to external services.
