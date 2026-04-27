# DAG Resolver

`pifp-dag-resolver` batches incoming user intents into dependency-aware Soroban
submission groups.

It does three things:

1. Builds a directed dependency graph from each intent's read and write sets.
2. Detects cycles and reports the exact conflicting intent path.
3. Produces topological layers so independent intents can be submitted in
   parallel.

## Input Format

The CLI accepts either a JSON array of intents or a wrapper object with an
`intents` field.

```json
[
  {
    "id": "mint-1",
    "reads": ["supply_cap"],
    "writes": ["total_supply"]
  },
  {
    "id": "notify-1",
    "reads": ["total_supply"]
  }
]
```

Each intent supports:

- `id`: unique identifier
- `reads`: resources that must stay stable while the intent executes
- `writes`: resources that the intent mutates
- `after`: optional explicit precedence constraints by intent id

## Usage

```bash
cargo run -p pifp-dag-resolver -- --intents intents.json
```

The resolver prints a JSON report describing:

- dependency edges
- parallel batches
- maximum parallel width
- any cycle that blocked resolution
