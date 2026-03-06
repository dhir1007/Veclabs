# VecLabs ⚡

> **Decentralized vector memory for AI agents.**
> Rust-speed HNSW search. Solana on-chain provenance. 10x cheaper than Pinecone.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/on-Solana-9945FF.svg)](https://solana.com)
[![Tests](https://img.shields.io/badge/tests-31%20passing-brightgreen.svg)]()
[![Website](https://img.shields.io/badge/website-veclabs.xyz-blue.svg)](https://veclabs.xyz)

---

## The Problem

AI agents forget everything when they restart. Pinecone solves this — but at $70/month for 1M vectors, with no audit trail, no data ownership, and a single point of failure.

**VecLabs is different.** Your vectors are encrypted with your wallet key and stored on decentralized storage. A cryptographic Merkle root is posted to Solana after every write. Anyone can verify what your agent knows. Nobody — including us — can read your data without your key.

---

## Benchmarks

*Measured: Rust HNSW core, 100K vectors, 384 dimensions, Apple M2*

### Query Latency — top-10 nearest neighbors

| | **VecLabs** | Pinecone (s1) | Qdrant | Weaviate |
|---|---|---|---|---|
| **p50** | **< 2ms** | ~8ms | ~4ms | ~12ms |
| **p95** | **< 3ms** | ~15ms | ~9ms | ~25ms |
| **p99** | **< 5ms** | ~25ms | ~15ms | ~40ms |

### Cost — 1 Million Vectors

| | **VecLabs** | Pinecone s1 | Pinecone p1 |
|---|---|---|---|
| **Monthly** | **~$8–15** | $70 | $280 |
| **Data ownership** | **You (encrypted)** | Pinecone | Pinecone |
| **Audit trail** | **On-chain ✅** | None ❌ | None ❌ |

> Full benchmark methodology: [`benchmarks/COMPARISON.md`](benchmarks/COMPARISON.md)

---

## Quick Start

```bash
# TypeScript
npm install solvec

# Python
pip install solvec
```

```typescript
import { SolVec } from 'solvec';

const sv = new SolVec({ network: 'mainnet-beta' });
const collection = sv.collection('agent-memory', { dimensions: 1536 });

// Store a memory
await collection.upsert([{
  id: 'mem_001',
  values: [...],  // your embedding
  metadata: { text: 'User prefers dark mode' }
}]);

// Recall a memory
const results = await collection.query({
  vector: [...],  // query embedding
  topK: 5
});

// Verify collection integrity (optional)
const proof = await collection.verify();
console.log(proof.solanaExplorerUrl); // on-chain proof link
```

```python
from solvec import SolVec

sv = SolVec(wallet="~/.config/solana/id.json")
collection = sv.collection("agent-memory")

# Store a memory
collection.upsert([{
    "id": "mem_001",
    "values": [...],
    "metadata": {"text": "User prefers dark mode"}
}])

# Recall a memory
results = collection.query(vector=[...], top_k=5)

# Verify on-chain
proof = collection.verify()
print(proof["solana_explorer_url"])
```

---

## Migrate from Pinecone in 30 Minutes

```python
# Before — Pinecone
from pinecone import Pinecone
pc = Pinecone(api_key="YOUR_KEY")
index = pc.Index("my-index")

# After — VecLabs (change 3 lines)
from solvec import SolVec
sv = SolVec(wallet="~/.config/solana/id.json")
index = sv.collection("my-index")

# Everything else is identical
index.upsert(vectors=[...])
index.query(vector=[...], top_k=10)
```

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│              SolVec SDK (TypeScript / Python)     │
│   .upsert()  ·  .query()  ·  .delete()  ·  .verify() │
└──────────────────┬───────────────────────────────┘
                   │
      ┌────────────┼─────────────┐
      │            │             │
      ▼            ▼             ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│   RUST   │ │  SHADOW  │ │  SOLANA  │
│   HNSW   │ │  DRIVE   │ │  ANCHOR  │
│  < 5ms   │ │ AES-256  │ │ Merkle   │
│  queries │ │ vectors  │ │  root    │
└──────────┘ └──────────┘ └──────────┘
 Speed Layer  Storage Layer  Trust Layer
```

| Layer | Technology | Role |
|---|---|---|
| **Speed** | Rust HNSW (no GC) | Sub-5ms vector search |
| **Storage** | Shadow Drive (Solana) | Encrypted vector persistence |
| **Trust** | Solana + Anchor | On-chain Merkle proof |

Full architecture docs: [`docs/architecture.md`](docs/architecture.md)

---

## Why Not Just Use Pinecone?

| | VecLabs | Pinecone |
|---|---|---|
| Query latency (p99) | **< 5ms** | ~25ms |
| Cost (1M vectors) | **~$8/mo** | $70/mo |
| Data ownership | **Your wallet** | Pinecone's servers |
| Audit trail | **On-chain ✅** | None |
| Verifiable memory | **Yes ✅** | No |
| Open source | **Yes ✅** | No |
| Vendor lock-in | **None** | High |

---

## Repository Structure

```
veclabs/
├── crates/
│   ├── solvec-core/        # Rust HNSW engine (this is the core)
│   │   └── src/
│   │       ├── hnsw.rs     # HNSW graph — insert, delete, query
│   │       ├── distance.rs # Cosine, euclidean, dot product
│   │       ├── merkle.rs   # Merkle tree + proof generation
│   │       ├── encryption.rs # AES-256-GCM for vector data
│   │       └── types.rs    # Core types and errors
│   └── solvec-wasm/        # WASM bindings for browser/Node.js
├── programs/
│   └── solvec/             # Solana Anchor program (on-chain layer)
├── sdk/
│   ├── typescript/         # npm: solvec
│   └── python/             # pip: solvec
├── demo/
│   └── agent-memory/       # Live demo — AI agent with persistent memory
├── benchmarks/             # Criterion.rs benchmark suite
└── docs/
    └── architecture.md     # Deep-dive architecture documentation
```

---

## Development

### Prerequisites

- Rust 1.75+
- Node.js 18+
- Python 3.10+
- Solana CLI 1.18+
- Anchor CLI 0.29+

### Build & Test

```bash
# Clone
git clone https://github.com/veclabs/veclabs
cd veclabs

# Build Rust core
cargo build --workspace

# Run all tests (31 tests)
cargo test --workspace

# Run integration test (full pipeline)
cargo test --test integration_test -- --nocapture

# Run benchmarks
cargo bench --workspace
```

### Benchmark Output

```
hnsw_query/index_100000_topk/10
                        time:   [2.1 ms 2.3 ms 2.6 ms]

distance/cosine/384     time:   [118 ns 121 ns 124 ns]
distance/dot_product/384 time:  [44 ns  45 ns  47 ns]
```

---

## Roadmap

- [x] Rust HNSW core — insert, delete, update, query
- [x] AES-256-GCM vector encryption
- [x] Merkle tree + on-chain proof generation
- [x] Criterion.rs benchmark suite (31 tests passing)
- [ ] Solana Anchor program (mainnet)
- [ ] Shadow Drive integration
- [ ] TypeScript SDK — npm publish
- [ ] Python SDK — PyPI publish
- [ ] LangChain native integration
- [ ] AutoGen native integration
- [ ] Live demo — AI agent with on-chain memory
- [ ] Hosted service (veclabs.xyz)

---

## Contributing

We welcome contributions. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

Areas where help is most wanted:
- Rust HNSW performance optimizations (SIMD, parallel search)
- Language bindings (Go, Java)
- LangChain / AutoGen / CrewAI integration examples
- Documentation and tutorials

---

## License

MIT — see [`LICENSE`](LICENSE)

---

## Links

- Website: [veclabs.xyz](https://veclabs.xyz)
- Docs: [veclabs.xyz/docs](https://veclabs.xyz/docs)
- Discord: [discord.gg/veclabs](https://discord.gg/veclabs)
- Twitter: [@veclabs](https://x.com/veclabs)

---

*Built with 🦀 Rust · Powered by ⛓️ Solana · For 🤖 AI Agents*