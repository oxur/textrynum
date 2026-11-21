# 0026 — Banyan Data Pipeline Requirements Analysis

**Date:** 2026-03-17
**Status:** Draft
**Purpose:** Document the complete data pipeline / workflow needs for four Banyan customers (Acme Inc., Affinity, Chime, Giant Eagle) such that each could be correctly reimplemented in ECL.

---

## 1. Executive Summary

Banyan operates a multi-tenant batch data processing platform built on GCP (Cloud Composer/Airflow for orchestration, Apache Beam/Dataflow for transformation, Kafka for output). Four customer pipelines are analyzed here to extract requirements for an ECL-based reimplementation:

| Customer | Partner ID | Complexity | Key Pattern |
|----------|-----------|------------|-------------|
| **Acme Inc.** | 327 | High | Multi-file retail receipts + loyalty matching |
| **Affinity** | 290 | Low | Single-file transaction normalization |
| **Chime** | 341 | Medium | BigQuery-driven rewards calculation |
| **Giant Eagle** | 296 | High | Multi-file retail receipts (ZIP compressed) |

---

## 2. Acme Inc. (Partner ID: 327)

### 2.1 Overview

Three sub-pipelines processing retail transaction data, loyalty consumer matching, and pharmacy confirmations.

### 2.2 Data Sources

| Source | Transport | Encryption | Format | Schedule |
|--------|----------|------------|--------|----------|
| Transaction files | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Payment/tender files | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Item files | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Store files | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Product files | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Digital variants | SFTP → GCS | PGP | CSV | Daily 9:30 PM UTC |
| Loyalty files | SFTP → GCS | PGP | CSV | Daily 8 PM UTC |
| RX confirmations | SFTP → GCS | PGP | PGP files | Every 15 min |

**File naming:** `wag_banyan_{type}_file[_digital]_YYYYMMDDHHMMSS.csv`

**SFTP bucket:** `byn-prod-merchant-327-wlgrns-sftp`
**Partner bucket:** `byn-prod-merchant-327-acme-inc`
**Loyalty bucket:** `byn-prod-merchant-327-acme-inc-loyalty`

### 2.3 CSV Schemas

#### Transaction CSV (8 columns)
| Column | Type | Notes |
|--------|------|-------|
| merchant_receipt_id | string | Primary key |
| store_id | string | FK to stores |
| merchant_total_amount | float | Divide by 100 for digital |
| merchant_id | string | |
| merchant_consumer_token | string | |
| order_type | string | |
| fulfillment_type | string | Mapped: in_store, ship |
| bopisFlag | string | Y/N, Buy Online Pickup In Store |

#### Tender CSV (11 columns)
| Column | Type | Notes |
|--------|------|-------|
| merchant_receipt_id | string | FK to transactions |
| shipping_amount | float | Divide by 100 for digital |
| tax_amount | float | Divide by 100 for digital |
| subtotal_amount | float | Divide by 100 for digital |
| payment_type | string | Mapped to canonical |
| scheme | string | Mapped: Visa→VI, MC→MC, etc. |
| card_last_four | string | |
| bin | string | |
| charged_amount | float | Divide by 100 for digital |
| authorization_timestamp | string | |
| AuthCode | string | Cleaned: strip leading zeros, left-pad to 6 |
| acquirer_reference_number | string | |
| PAR | string | Payment Account Reference |

#### Item CSV (8 columns)
| Column | Type | Notes |
|--------|------|-------|
| merchant_receipt_id | string | FK to transactions |
| quantity | int | |
| full_price_per_unit | float | |
| discount_price_per_unit | float | |
| amount_paid | float | |
| upc | string | Needs check digit calculation |
| upc_description | string | |
| fsa_item_ind | string | FSA indicator |

#### Store CSV (9 columns)
| Column | Type | Notes |
|--------|------|-------|
| store_id | string | Primary key |
| merchant_name | string | |
| card_acceptor_name | string | |
| address_line_one | string | |
| address_line_two | string | |
| city | string | |
| state | string | |
| postal_code | string | Used for timezone lookup |

#### Product CSV (12 columns)
| Column | Type | Notes |
|--------|------|-------|
| upc | string | Join key to items |
| description | string | |
| category_id | string | |
| category_name | string | |
| brand | string | |
| mdse_cat_name | string | |
| MDSE_DIV_NAME | string | |
| PROD_SK | string | Optional |
| PROD_REPORT_DESC | string | |
| ITEM_DESC | string | |
| PRIVATE_LABEL_IND | string | |
| OPS_DEPT_NAME | string | |
| PROD_ID | string | |

#### Loyalty CSV (6 columns)
| Column | Type | Notes |
|--------|------|-------|
| loyalty_id_sha | string | SHA256 hash, primary key |
| home_ph_sha | string | Mutually exclusive with other phones |
| cell_ph_sha | string | |
| work_ph_sha | string | |
| email_sha | string | |
| Flag | string | Optional |

### 2.4 Transformation Pipeline (Receipt Data)

**Phase 1: File Management**
1. Storage Transfer Service: SFTP bucket → partner bucket `input/`
2. PGP decryption (4 parallel workers via Cloud Function)
3. Deduplication by MD5 hash (group by file_type+date, keep newest)
4. Move from `input/` → `staging/` (last 2 days only)
5. Validate staging folder non-empty

**Phase 2: Data Transformation (Apache Beam)**

Step-by-step transformations:

1. **CSV Parsing** — Read 5 file types with distinct schemas
2. **Price Correction (Digital only)** — Divide amounts by 100 (stored as cents)
3. **Order Type Mapping:**
   - Regular: map fulfillment_type → standardized values
   - Digital: force order_type='web', fulfillment_type='ship'
   - BOPIS: force order_type='web', fulfillment_type='in_store'
4. **Payment Type Mapping:**
   - Regular: Credit Card→CREDIT, Debit Card→DEBIT, EBT→EBT, Cash→CASH, etc.
   - Digital: brpoints→COUPON, waggc→GIFT CARD, fsacc→CREDIT, paypal→OTHER
5. **Payment Scheme Mapping:** Visa/V→VI, Mastercard/M→MC, Amex→AX, Discover/D→DS
6. **UPC Check Digit Calculation** — GS1 algorithm for 12→13 digit conversion
7. **Auth Code Cleaning** — Strip Medagate leading zeros, left-pad to 6 digits
8. **Tender Aggregation** — Group by receipt_id; aggregate amounts; compute cash_back
9. **Store Timezone Resolution** — ZIP code → timezone; store 5995 → UTC
10. **Item-Product Join** — CoGroupByKey on UPC; enrich items with product data
11. **Hot Key Handling** — UPC 999999999999 processed separately, re-merged
12. **Validation** — Transaction, Tender, Item, Store, Product validators
13. **Receipt Assembly** — Merge all entities into canonical receipt structure

**Phase 3: Output**
- Write receipts to Kafka topic (Avro): `by_production_327_acme_inc_receipt-preprocess_canonical_batch_avro_0`
- Write taxonomy to Kafka topic (Avro): `by_production_acme_inc_taxonomy_avro_0`
- Move successful files to historical archive
- Move failed files back to `input/`
- Write validation errors to `error/` folder as CSV

### 2.5 Loyalty Pipeline

**Phase 1: File Management**
- Sense loyalty files in SFTP
- Verify upload complete (check 0-byte)
- Move SFTP → partner bucket
- PGP decrypt

**Phase 2: Transformation (Kubernetes worker)**
1. Split large files (>2GB → 1GB chunks)
2. Load into PostgreSQL `acme_inc_loyalty` table (50k records/batch)
3. Compare staged vs existing CATs
4. Compute diffs:
   - NULL→NOT NULL: send insert (new consumer token)
   - NOT NULL→NULL: send delete
   - Different values: send update
   - Duplicates: send delete (fraud prevention)
5. Hash algorithm: SHA256(salt + SHA256(phone)) → `cat_3_1_{hash}`

**Phase 3: Output**
- Kafka topic (Avro): `by_{env}_mcon-accept-batch_event_avro_0`
- Messages: `{byn_partner_id: 327, consumer_token: loyalty_id_sha, ats: [cat_values]}`
- Empty ats array for deletes

### 2.6 Error Handling
- Hard/soft validation failures → separate CSV files in `error/`
- Failed files return to `input/` for retry
- MD5 deduplication prevents reprocessing identical files
- Loyalty: database-based idempotent design

### 2.7 Special Handling / Quirks
- BOPIS flag detection
- Digital file pricing in cents
- Store 5995 uses UTC (online orders)
- Debit Card/Cashback negative amounts filtered
- Medagate auth code special cleaning
- Hot key UPC 999999999999
- 0-byte file detection during upload
- Loyalty: only ONE phone field per record

---

## 3. Affinity (Partner ID: 290)

### 3.1 Overview

Simple single-file transaction normalization pipeline. Currently paused in production.

### 3.2 Data Source

| Source | Transport | Encryption | Format | Schedule |
|--------|----------|------------|--------|----------|
| Transaction files | GCS (STS) | None | CSV | Mon/Wed-Sun 3 PM UTC |

**File pattern:** `Banyan-txn-file*`
**Bucket:** `byn-prod-finx-290-affinity`

### 3.3 CSV Schema (19 columns)

| Column | Type | Mapping |
|--------|------|---------|
| Channel_Aggregator_ID | string | (unused) |
| Program_ID | string | (unused) |
| Account_ID | string | → finx_consumer_token |
| Card_BIN | string | (unused) |
| Card_Last_Four | string | → payment.card_last_four (validated: 4 digits) |
| Account_Postal_Code | string | (unused) |
| Merchant_Location_MID | string | → payment.finx_mid |
| Merchant_Location_Name | string | → merchant.name, location.name |
| Merchant_Location_Street | string | → location.address.address_line_one |
| Merchant_Location_City | string | → location.address.city |
| Merchant_Location_State | string | → location.address.state |
| Merchant_Location_Postal_Code | string | → location.address.postal_code |
| Merchant_Location_Category_Code | string | (unused) |
| Transaction_ID | string | → finx_transaction_id (validated: not null) |
| Transaction_Date | string | → purchase_ts (MM/DD/YYYY → UTC datetime) |
| Transaction_Settlement_Date | string | (unused) |
| Transaction_Code | string | (unused) |
| Transaction_Amount | string | → payment.total_amount |
| Transaction_Auth_Code | string | → payment.auth_code (padded to 6 digits) |

### 3.4 Transformation Pipeline

**Phase 1: File Management**
1. STS job: copy files to `input/`
2. Move from `input/` → `staging/{timestamp}/`
3. Validate staging non-empty

**Phase 2: Data Transformation (Apache Beam)**
1. **CSV Parse** — 19-column schema
2. **Date Parse** — MM/DD/YYYY → UTC datetime
3. **Auth Code Pad** — Left-pad to 6 digits with zeros
4. **Store ID Extract** — Regex on merchant name: `ACME INC\.\s*#\s*(\d+)`
5. **Add Job Metadata** — job_id, start_time, source
6. **Map to Canonical** — Transform to standard transaction output format
7. **Validate** — Last4 (4 digits), Transaction ID (not null), Timestamps (±10 years)

**Phase 3: Output**
- Kafka topic (Avro): `by_production_transaction_event_avro_0`
- Sandbox only: JSON files to GCS `output/`
- Validation errors to `error/` (per error type)
- Successful files → historical archive
- Failed files → back to `input/`

### 3.5 Validation Rules
- `card_last_four`: exactly 4 digits
- `finx_transaction_id`: must not be null
- Timestamps: all within ±10 years of now

### 3.6 Key Characteristics
- **Simplest pipeline** — single file type, minimal transformations
- No encryption
- No joins or aggregations
- Fixed field mapping with some computed fields
- Country always "US", currency always "USD"

---

## 4. Chime (Partner ID: 341)

### 4.1 Overview

Campaign-based rewards calculation pipeline. Identifies eligible Acme Inc. transactions made with Chime cards, calculates tiered cashback rewards.

### 4.2 Data Sources

| Source | Type | Notes |
|--------|------|-------|
| ChimeBins CSV | GCS file | List of Chime BIN numbers (6-digit) |
| receipt_payment | BigQuery table | Acme Inc. transaction payments |
| dim_receipt | BigQuery table | Receipt dimension data |
| dim_receipt_item | BigQuery table | Line item details with tags |
| chime_rewards | BigQuery table | Previously calculated rewards (dedup) |

**Schedule:** Daily 2 PM UTC (9 AM EST)
**Campaign period:** 2025-07-20 to 2025-11-16 (production)

### 4.3 Pipeline Flow

1. **Check Input** — Validate BIN files exist in `input/ChimeBins/`
2. **Collect BINs** — Parse CSV files, deduplicate, sort
3. **Calculate Rewards** — BigQuery SQL query:
   - Filter: partner_id=327, BIN in list, date in campaign range
   - Exclude: receipts already in chime_rewards table
   - Join: receipt_payment ↔ dim_receipt ↔ dim_receipt_item
   - Categorize: private_label vs non-private_label items
   - Calculate: `private_label_amt * 0.05 + non_private_label_amt * 0.03`
   - Cap: LEAST(reward, $5.00) per receipt
   - Proportional distribution across split payments
4. **Output CSV** — Write to GCS
5. **Copy to S3** — Transfer to partner's S3 bucket

### 4.4 Output Schema

| Column | Type | Description |
|--------|------|-------------|
| byn_receipt_id | string | Receipt identifier |
| byn_payment_id | string | Payment identifier |
| auth_code | string | Authorization code |
| bin | string | Bank ID number |
| card_last_four | string | Last 4 digits |
| purchased_ts | timestamp | Purchase timestamp |
| payment_total_amount | numeric | Payment amount |
| total_reward_amount | numeric | Calculated reward |

### 4.5 Key Characteristics
- **SQL-driven** — core logic is BigQuery SQL, not streaming
- **Cross-pipeline data dependency** — reads Acme Inc. (327) data
- **Campaign-bounded** — has start/end dates
- **Multi-output** — GCS + S3
- **Dedup via database** — tracks processed receipts
- **No file transformation** — query results written directly

---

## 5. Giant Eagle (Partner ID: 296)

### 5.1 Overview

Multi-file retail receipt pipeline similar to Acme Inc. but with ZIP compression instead of PGP encryption.

### 5.2 Data Sources

| Source | Transport | Compression | Format | Schedule |
|--------|----------|-------------|--------|----------|
| Store files | GCS (STS) | ZIP | CSV (no headers) | Daily 3 PM UTC |
| Transaction files | GCS (STS) | ZIP | CSV (with headers) | Daily 3 PM UTC |
| Item files | GCS (STS) | ZIP | CSV (with headers) | Daily 3 PM UTC |
| Tender files | GCS (STS) | ZIP | CSV (with headers) | Daily 3 PM UTC |
| Product files | GCS (STS) | ZIP | CSV (no headers) | Daily 3 PM UTC |

**Bucket:** `byn-prod-merchant-296-giant-eagle-inc`

### 5.3 CSV Schemas

#### Store CSV (12 columns, no headers)
MERCHANT_NAME, STORE_ID, NAME, DISPLAY_NAME, STREET_ADDRESS_1, STREET_ADDRESS_2, CITY, STATE, POSTAL_CODE, COUNTRY, PHONE_NUMBER, BS_STR_NO

#### Transaction CSV (12 columns, with headers)
TRANSACTION_ID, PURCHASE_TIMESTAMP, STORE_ID, NUMBER_OF_ITEMS, CONSUMER_TOKEN, FULLFILMENT_TYPE (sic), SHIPPING_AMOUNT, TOTAL_AMOUNT, TAX_AMOUNT, SUBTOTAL_AMOUNT, RECEIPT_BARCODE_URL, RECEIPT_IMAGE_URL

#### Item CSV (11 columns, with headers)
TRANSACTION_ID, ITEM_ID, SKU, UPC, UNIT_TYPE, FULL_PRICE_PER_UNIT, QUANTITY, SUBTOTAL, SALE_UNITS, DISCOUNT_PRICE_PER_UNIT, AMOUNT_PAID

#### Tender CSV (10 columns, with headers)
TRANSACTION_ID, PURCHASE_TIMESTAMP, MERCHANT_ID, PAYMENT_TYPE, PAYMENT_SCHEME, PAYMENT_FOUR, BIN, PAYMENT_AMT, ARN, AUTHORIZATION_CODE

#### Product CSV (10 columns, no headers, has unescaped quotes)
SKU, UPC, BRAND, DESCRIPTION, ITEM_IMAGE_URL, CATEGORY_NAME, CATEGORY_ID, DEPARTMENT_NAME, DEPARTMENT_ID, HIERARCHY_ITEM

### 5.4 Transformation Pipeline

**Phase 1: File Management**
1. STS job: copy files to `input/`
2. ZIP decompression (4 workers via Cloud Function)
3. Move from `input/` → `staging/` (last 1 day)
4. Validate staging non-empty

**Phase 2: Data Transformation (Apache Beam)**
1. **CSV Parse** — 5 file types with distinct schemas
2. **Timestamp Parse** — Local time with timezone lookup by store ZIP
3. **Fulfillment Mapping:**
   - IN STORE, PICKUP → in_store
   - DELIVERED, INSTACART → delivery
4. **Payment Type Mapping** — Similar to Acme Inc.
5. **Payment Scheme Mapping** — Visa→VI, Mastercard→MC, Discover→DS, etc.
6. **Cash Back Aggregation** — Negative cash payments → cash_back_amount on highest payment
7. **UPC Check Digit Validation**
8. **Unit Type Standardization** — EA→each, LB→lb, GL→gallon
9. **Item-Product Join** — By UPC/SKU; enrich with brand, category, department
10. **Tender Aggregation** — Group by transaction; proportional amount distribution
11. **Receipt Assembly** — Merge transaction + store + tenders + items
12. **Taxonomy Extraction** — Product catalog normalization
13. **Validation** — Hard/soft validation on all entities
14. **Merchant Name Logic** — GET GO vs GIANT EAGLE based on store data

**Phase 3: Output**
- Receipts → Kafka topic (Avro): `by_production_296_giant-eagle-inc_receipt-accept_canonical_batch_avro_0`
- Historical → Kafka topic (Avro): `by_production_296_giant-eagle-inc_receipt-accept_canonical_batch-historical_avro_0`
- Taxonomy → Kafka topic (Avro): `by_production_merchant-taxonomy_event_avro_0`
- Validation errors → GCS `error/` as CSV
- Successful files → historical archive
- Failed files → back to `input/`

### 5.5 Key Characteristics
- ZIP compression (not PGP encryption)
- Product CSV has unescaped quotes (parsing challenge)
- Fuel prepay logic exists but is commented out
- GET GO sub-brand detection
- Similar complexity to Acme Inc. but different field mappings

---

## 6. Cross-Cutting Patterns

### 6.1 Common Pipeline Lifecycle

All pipelines (except Chime) follow this pattern:

```
Source → Ingest → Stage → [Decrypt/Decompress] → Transform → Validate → Output → Archive
```

Specifically:
1. **Ingest:** Move files from external source to `input/` folder
2. **Stage:** Move from `input/` to `staging/{timestamp}/`
3. **Pre-process:** Decrypt (PGP) or decompress (ZIP) if needed
4. **Transform:** Parse CSV, apply business logic, join data streams
5. **Validate:** Check data quality (hard/soft failures)
6. **Output:** Write to Kafka (Avro) and/or files (CSV/JSON)
7. **Archive:** Move processed → historical; failed → input for retry

### 6.2 Common Capabilities Required

| Capability | Acme Inc. | Affinity | Chime | Giant Eagle |
|-----------|-----------|----------|-------|-------------|
| GCS source | Yes | Yes | Yes | Yes |
| SFTP source | Yes | No | No | No |
| S3 output | No | No | Yes | No |
| CSV parsing | Yes | Yes | Yes | Yes |
| PGP decryption | Yes | No | No | No |
| ZIP decompression | No | No | No | Yes |
| Multi-file joins | Yes | No | No | Yes |
| Aggregation | Yes | No | No | Yes |
| Kafka output (Avro) | Yes | Yes | No | Yes |
| BigQuery query | No | No | Yes | No |
| Field mapping | Yes | Yes | Yes | Yes |
| Date/time parsing | Yes | Yes | No | Yes |
| Timezone conversion | Yes | No | No | Yes |
| Payment type mapping | Yes | No | No | Yes |
| UPC check digit | Yes | No | No | Yes |
| Deduplication | Yes | No | Yes (DB) | No |
| Validation rules | Yes | Yes | No | Yes |
| File lifecycle mgmt | Yes | Yes | No | Yes |
| Scheduling | Yes | Yes | Yes | Yes |
| Observability | Yes | Yes | Yes | Yes |
| Secret management | Yes | Yes | Yes | Yes |

### 6.3 Data Standards

From the existing documentation:
- **Dates:** RFC3339 with UTC timezone
- **Amounts:** 2 decimal places
- **Currency:** ISO 4217 (3-letter uppercase)
- **Country:** ISO 3166-1 alpha-3
- **Source attribution:** BYN (generated), SRC (from partner), SRC,BYN (fallback)

### 6.4 Canonical Output Model

All receipt-producing pipelines (Acme Inc., Affinity, Giant Eagle) output to a shared canonical schema:

```
Receipt:
  transaction:
    id, purchase_ts, authorization_ts, received_ts
    byn_partner_id
    merchant: {id, name, byn_partner_id}
    location: {name, display_name, merchant_store_id, phone_number, address: {...}}
    payment: {payment_type, scheme, card_last_four, bin, auth_code, arn, total_amount, currency}
    order_type, description
    finx_transaction_id, finx_consumer_token
    items: [{quantity, full_price_per_unit, discount_price_per_unit, amount_paid, upc, ...}]
  meta:
    created_ts, entity_type, producer_host
    file_details: {filename, line_number}
```

---

## 7. Requirements Summary for ECL Implementation

### 7.1 Source Adapters Needed

| Adapter | Priority | Used By |
|---------|----------|---------|
| GCS (Google Cloud Storage) | P0 | All four customers |
| SFTP | P1 | Acme Inc. |
| S3 (AWS) | P1 | Chime (output) |
| BigQuery | P2 | Chime (input) |

### 7.2 Sink/Output Adapters Needed

| Adapter | Priority | Used By |
|---------|----------|---------|
| GCS file writer | P0 | All (error output, archives) |
| Kafka (Avro) | P0 | Acme Inc., Affinity, Giant Eagle |
| S3 file writer | P1 | Chime |
| CSV file writer | P0 | Error reporting, Chime output |

### 7.3 Transformation Stages Needed

| Stage | Priority | Used By |
|-------|----------|---------|
| CSV parsing (configurable schema) | P0 | All |
| Field mapping/renaming | P0 | All |
| Date/time parsing (multiple formats) | P0 | WAG, AFF, GE |
| Timezone conversion (ZIP→TZ) | P1 | WAG, GE |
| Lookup table mapping (payment types, schemes) | P0 | WAG, GE |
| PGP decryption | P1 | WAG |
| ZIP decompression | P1 | GE |
| Join (CoGroupByKey equivalent) | P0 | WAG, GE |
| Aggregation (group by key, compute) | P0 | WAG, GE |
| Validation (configurable rules) | P0 | WAG, AFF, GE |
| UPC check digit calculation | P2 | WAG, GE |
| Auth code normalization | P1 | WAG, AFF, GE |
| Expression evaluation (computed fields) | P1 | All |
| Deduplication (by hash or key) | P1 | WAG |
| Split/fan-out (conditional routing) | P0 | All |

### 7.4 Pipeline Infrastructure Needed

| Feature | Priority | Notes |
|---------|----------|-------|
| Multi-source pipelines | P0 | Multiple CSV file types per pipeline |
| File lifecycle management | P0 | input → staging → historical/error |
| Scheduling (cron) | P0 | Varied schedules per customer |
| Pipeline chaining (trigger) | P1 | Ingestion → transformation |
| Secret management | P0 | Credentials for GCS, Kafka, PGP keys |
| Avro serialization | P0 | Kafka output format |
| Schema Registry integration | P0 | Confluent Schema Registry |
| Environment-specific config | P1 | sand/test/prod overrides |
| Observability (tracing) | P0 | Already in ECL |
| Error classification (hard/soft) | P1 | Validation error routing |

---

## 8. Complexity Assessment

### Affinity — Simplest (Good Starting Point)
- Single CSV file
- Linear pipeline (no joins)
- Simple field mapping + validation
- Standard Kafka output

### Chime — Medium (Different Pattern)
- SQL-based, not file-streaming
- Cross-pipeline dependency (reads Acme Inc. data)
- Campaign-bounded
- File output (no Kafka)

### Acme Inc. Receipt — High
- 5+ CSV file types
- Complex joins and aggregations
- PGP decryption
- Extensive validation
- Multiple output topics

### Giant Eagle — High
- 5 CSV file types
- ZIP decompression
- Complex joins and aggregations
- Product data with parsing issues
- Multiple output topics

### Acme Inc. Loyalty — Medium-High
- Stateful processing (PostgreSQL)
- Hashing/tokenization
- Diff-based updates
- Avro/Kafka output

**Recommended implementation order:** Affinity → Chime → Giant Eagle → Acme Inc.

---

## Appendix A: Configuration Examples

### A.1 Acme Inc. Production Config (excerpt)
```yaml
acme-inc:
  partner_id: 327
  kafka_topic: by_production_327_acme_inc_receipt-preprocess_canonical_batch_avro_0
  partner_bucket_name: byn-prod-merchant-327-acme-inc
  partner_sftp_bucket_name: byn-prod-merchant-327-wlgrns-sftp
  dataflow:
    num_workers: 5
    max_workers: 300
  cloud_function:
    decrypt:
      encryption: PGP
      secret_key_name: private-pgp-327-acme-inc
      secret_pass_name: private-pgp-pass-327-acme-inc
  composer:
    transformation:
      last_n_days: 2
      decrypt_workers: 4
```

### A.2 Affinity Production Config (excerpt)
```yaml
affinity:
  partner_id: 290
  partner_bucket_name: byn-prod-finx-290-affinity
  dataflow:
    num_workers: 1
    max_workers: 100
```

### A.3 Chime Production Config (excerpt)
```yaml
chime:
  partner_id: 341
  partner_bucket_name: byn-prod-finx-341-chime
  composer:
    campaign_start_date: 2025-07-20
    campaign_end_date: 2025-11-16
```

### A.4 Giant Eagle Production Config (excerpt)
```yaml
giant_eagle_inc:
  partner_id: 296
  partner_bucket_name: byn-prod-merchant-296-giant-eagle-inc
  dataflow:
    num_workers: 1
    max_workers: 100
  cloud_function:
    decompress:
      workers: 4
```
