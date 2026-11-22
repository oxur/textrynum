# Baseline writer fixture oracle

Source checkout: `edc8db69a2ef45d49e1c369ad5cf489dc58d161b`; writer lock Git blob SHA-1: `244e0628cd90ca0a0e3dee3354cc020e31d6fe55`; writer toolchain: Rust/Cargo 1.95.0. The expected values below are authored independently of upgraded readers.

## Redb

`redb/state.redb` was written through `RedbStateStore::open`, `save_checkpoint`, and `save_completed_hashes`. Expected latest checkpoint: run `cc-fixture-run-42`, sequence 42, version 1, spec hash `spec-hash-cc-fixture-42`, pipeline `cc-redb-fixture`, running at `extract`, one scheduled batch (`extract`), fixed timestamp `2026-03-13T10:00:00Z`, source `local`, counters discovered 2 / accepted 1 / unchanged 1, and completed item `file-17` named `acme-report.txt` with content hash `sha256-independent-fixture-17`, filesystem provenance and `fixture=old-writer`. Stats: discovered 2, processed 1, unchanged 1, failed 0. The saved completed hash map is exactly `acme-report.txt -> golden-hash-acme-17` and `schedule.json -> golden-hash-schedule-23`. These assertions are the control oracle for an upgraded product reader.

## LanceDB

`lance/lance-db` was created by product `LancedbBackend::build`, table `cc_vectors`, with exactly three rows and the product schema: `id` Utf8 non-null, `text` Utf8 non-null, `category` nullable Utf8, `metadata` Utf8 non-null JSON, and `vector` fixed-size list of four Float32s with nullable Float32 child named `item`. Expected rows, independent of search order: `doc-null`, text `Document with nullable category`, null category, metadata map `{owner: acme}`, vector `[0,0,0,0]`; `doc-one`, text `One vector row`, category `reports`, metadata map `{owner: acme}`, vector `[1,1,1,1]`; `doc-two`, text `Two vector row`, category `archive`, metadata map `{owner: example}`, vector `[2,2,2,2]`. Query input is a four-zero vector. Squared L2 distance oracle is 0, 4, 16 and the product score oracle is 1, 1/5, 1/17 within floating-point tolerance. Product metadata JSON is exactly represented by `index_metadata.json`: hash `golden-lance-content-hash-42`, count 3, dimension 4, built-at `2026-03-13T10:00:00Z`, provider `cc-zero-oracle`, model `fixed-4d-fixture-v1`. The old writer did not execute a vector query and did not seed an unrelated table; those behaviors are separately tested after the upgrade/rebuild.

## Tantivy

The closed product disk index has three records. `doc-harmony`: title `Harmonics and harmony`, content `Harmonics support harmonic progressions`, category `harmony`, author `A. Example`, type `concept`, source `Acme Inc. fixture`, path `fixture/doc-harmony.md`, section `stored-section`. `doc-rhythm`: title `Rhythmic patterns`, content `A rhythmic pulse organizes measures`, category `rhythm`, author `B. Example`. `doc-cadence`: title `Cadence resolution`, content `Harmonic cadences resolve phrases`, category `harmony`, author `C. Example`. Queries after upgrade must demonstrate IDs/fields, category filtering, and English stemming (e.g. `harmonics` matching `harmony`/`harmonic` indexed text) against a copy.

`sha256-manifest.json` records immutable binary originals immediately after the successful writer runs. After each upgraded read/rebuild, recompute and compare every original hash; only owned copies may change.
