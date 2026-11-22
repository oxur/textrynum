use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::panic::{AssertUnwindSafe, catch_unwind};
use fabryk_content::markdown::parser::extract_first_paragraph;
use fabryk_content::metadata::{ContentType, extract_metadata};
use fabryk_content::sources::{
    SourceCategory, SourceResolver, ValidationMode, validate_sources,
};
use fabryk_fts::{
    ConceptCardDocumentExtractor, IndexBuilder, Indexer, SearchBackend, SearchConfig,
    SearchDocument, SearchParams, SearchSchema, SimpleSearch, TantivySearch,
    create_search_backend,
};
use fabryk_fts::types::QueryMode;
use fabryk_core::util::ids::normalize_id as core_normalize_id;

fn emit(key: &str, value: impl std::fmt::Display) { println!("{key}={value}"); }
fn doc(id: &str, title: &str, content: &str, category: &str) -> SearchDocument {
    SearchDocument::builder().id(id).title(title).content(content).category(category).build()
}
fn write(path: &Path, content: &str) { fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, content).unwrap(); }
fn index_docs(path: &Path, docs: &[SearchDocument]) {
    let schema = SearchSchema::build();
    let mut writer = Indexer::new(path, &schema).unwrap();
    for d in docs { writer.add_document(d).unwrap(); }
    writer.commit().unwrap();
}
fn tantivy(path: &Path, mut config: SearchConfig) -> TantivySearch {
    config.index_path = Some(path.to_string_lossy().into_owned());
    TantivySearch::new(&config).unwrap()
}
fn ids(results: &fabryk_fts::SearchResults) -> String {
    let mut v: Vec<_> = results.items.iter().map(|r| r.id.clone()).collect(); v.sort(); v.join(",")
}

#[tokio::main]
async fn main() {
    // 1. Unicode bounds and public parsing/snippet APIs.
    let p = catch_unwind(|| extract_first_paragraph("# T\n\néxample body", 1));
    emit("paragraph_unicode_limit1_panics", p.is_err());
    emit("paragraph_ascii_limit", extract_first_paragraph("# T\n\nalpha beta", 5).unwrap());
    emit("paragraph_zero_limit", extract_first_paragraph("# T\n\nalpha", 0).unwrap());
    emit("paragraph_exact_limit", extract_first_paragraph("# T\n\nalpha", 5).unwrap());
    let unicode_doc = doc("snippet", "Acme note", "foo\u{2003}bar baz", "general");
    emit("document_emspace_snippet_panics", catch_unwind(AssertUnwindSafe(|| unicode_doc.extract_snippet("baz", 4))).is_err());
    let expansion_doc = doc("expansion", "Acme note", "İx begins; needle is later in the text", "general");
    let shifted = doc("shifted", "Acme", "İ0123456789needleABCDEFGHIJ", "general");
    emit("document_dotted_i_shifted_excerpt", shifted.extract_snippet("needle", 8).unwrap_or_default());
    let ascii_shifted = doc("ascii", "Acme", "0123456789needleABCDEFGHIJ", "general");
    emit("document_ascii_shifted_control", ascii_shifted.extract_snippet("needle", 8).unwrap_or_default());
    for n in [4usize, 8, 12, 16] {
        let found = catch_unwind(AssertUnwindSafe(|| expansion_doc.extract_snippet("needle", n)));
        emit(&format!("document_dotted_i_snippet_limit_{n}"), match found { Ok(v) => v.unwrap_or_default(), Err(_) => "PANIC".into() });
    }
    let empty_snippet = doc("empty", "Acme", "", "general");
    emit("empty_content_snippet", empty_snippet.extract_snippet("missing", 0).unwrap_or_default());

    for (name, input) in [
        ("frontmatter_prefix_open_close", "---opening note\ntitle: Acme\n---trailing note\n# Body\n"),
        ("frontmatter_malformed", "---\ntitle: [bad\n---\n# Body\n"),
        ("frontmatter_empty", "---\n---\nBody\n"),
        ("frontmatter_no_close", "---\ntitle: Acme\nBody\n"),
    ] {
        let r = fabryk_content::extract_frontmatter(input).unwrap();
        emit(&format!("{name}_delimiters"), r.had_delimiters());
        emit(&format!("{name}_has_value"), r.value().is_some());
        emit(&format!("{name}_body"), r.body().replace('\n', "|"));
    }
    emit("extract_text_fenced_dash", fabryk_content::extract_text_content("```yaml\n---\nkey: Acme\n```\n# Body\n"));

    // 2. Identity and competing metadata policies.
    for s in [" A__  B\tC ", "A   B", "A_B"] {
        emit(&format!("id_core_{s:?}"), core_normalize_id(s));
        emit(&format!("id_content_{s:?}"), fabryk_content::normalize_id(s));
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("concept-cards");
    let file = root.join("nested").join("Acme_File.md");
    write(&file, "---\nchapter: Chapter fallback\nsource: Acme Inc.\nauthor: Acme Research\ndate: 2026-11-22\nsection: A.1\naliases: [Alias One]\n---\n# Body heading\n\nBody text.\n");
    let metadata = extract_metadata(&root, &file, ContentType::ConceptCard).await.unwrap();
    let extracted = ConceptCardDocumentExtractor::new().extract(&file, &fs::read_to_string(&file).unwrap()).unwrap();
    emit("metadata_id_title_category_source_section_author_date", format!("{}|{}|{}|{:?}|{:?}|{:?}|{:?}", metadata.id, metadata.title, metadata.category, metadata.source, metadata.section, metadata.author, metadata.date));
    emit("fts_id_title_category_source_section_content_type_author_date", format!("{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}", extracted.id, extracted.title, extracted.category, extracted.source, extracted.section, extracted.content_type, extracted.author, extracted.date));
    emit("fts_content_has_frontmatter", extracted.content.contains("chapter:") || extracted.content.contains("source:"));
    let mut cats = HashMap::new();
    cats.insert("a-b".to_string(), SourceCategory { path: "".into(), files: [("c".into(), "[2020] Acme - Shared.pdf".into())].into(), aliases: HashMap::new() });
    cats.insert("a".to_string(), SourceCategory { path: "".into(), files: [("b-c".into(), "[2020] Acme - Shared.pdf".into())].into(), aliases: HashMap::new() });
    cats.insert("x".to_string(), SourceCategory { path: "".into(), files: [("one".into(), "[2020] Acme - Alias Clash.pdf".into())].into(), aliases: HashMap::new() });
    cats.insert("y".to_string(), SourceCategory { path: "".into(), files: [("two".into(), "[2020] Acme - Distinct.pdf".into())].into(), aliases: [("two".into(), vec!["Alias Clash".into()])].into() });
    cats.insert("z".to_string(), SourceCategory { path: "".into(), files: [("three".into(), "[2020] Acme - Third.pdf".into())].into(), aliases: [("three".into(), vec!["Alternate Alias".into()])].into() });
    cats.get_mut("x").unwrap().aliases.insert("one".into(), vec!["Alternate Alias".into()]);
    let resolver = SourceResolver::from_categories(&cats);
    let shared = resolver.resolve("Shared");
    let alias_title_collision = resolver.resolve("Alias Clash");
    let alias_only = resolver.resolve("Alternate Alias");
    emit("collision_known_ids", { let mut x: Vec<_> = resolver.known_ids().iter().cloned().collect(); x.sort(); x.join(",") });
    emit("collision_shared_resolution", format!("{:?}|{:?}", shared.0, shared.1));
    emit("title_over_alias_resolution", format!("{:?}|{:?}", alias_title_collision.0, alias_title_collision.1));
    emit("ambiguous_alias_resolution", format!("{:?}|{:?}", alias_only.0, alias_only.1));

    // Public source validation modes and filesystem edge cases.
    let vtmp = tempfile::tempdir().unwrap();
    let cards = vtmp.path().join("cards"); fs::create_dir_all(&cards).unwrap();
    write(&cards.join("valid.md"), "---\nsource: Acme Source\n---\n# Card\n");
    write(&cards.join("malformed.md"), "---\nsource: [bad\n---\n# Bad\n");
    let srcdir = vtmp.path().join("sources"); fs::create_dir_all(&srcdir).unwrap();
    write(&srcdir.join("[2020] Acme - Acme Source.pdf"), "fixture"); fs::create_dir_all(srcdir.join("directory.pdf")).unwrap();
    let mut vc = HashMap::new();
    vc.insert("general".into(), SourceCategory { path: srcdir.to_string_lossy().into_owned(), files: [
        ("valid".into(), "[2020] Acme - Acme Source.pdf".into()), ("absent".into(), "absent.pdf".into()), ("dir".into(), "directory.pdf".into())
    ].into(), aliases: HashMap::new() });
    for mode in [ValidationMode::All, ValidationMode::CardsConfig, ValidationMode::CardsFilesystem, ValidationMode::ConfigFilesystem] {
        let report = validate_sources(&cards, &vc, mode, true, 0.6).await.unwrap();
        emit(&format!("validation_{mode:?}"), format!("valid={};cards={};sources={};disk={};missing_fs={};missing_cfg={}", report.is_valid(), report.stats.total_cards_scanned, report.stats.sources_in_config, report.stats.sources_on_disk, report.stats.missing_from_disk, report.stats.missing_from_config));
        emit(&format!("validation_{mode:?}_missing_paths"), report.missing_from_filesystem.iter().map(|m| m.expected_path.display().to_string()).collect::<Vec<_>>().join(","));
    }
    let no_cards = validate_sources(&cards.join("missing"), &vc, ValidationMode::ConfigFilesystem, false, 0.6).await;
    emit("config_filesystem_missing_content_root_errors", no_cards.is_err());

    // 4/5. Actual Tantivy public search semantics, ranking, filtering, counts, snippets, limit.
    let qtmp = tempfile::tempdir().unwrap(); let qidx = qtmp.path().join("qindex");
    index_docs(&qidx, &[
        doc("split", "Acme alpha", "beta body", "wanted"),
        doc("phrase-only", "Acme circle of fifths", "circle of fifths", "wanted"),
        doc("phrase-adjacent", "Acme circle fifths", "circle fifths", "wanted"),
        doc("cadence-only", "Acme cadence", "cadence", "wanted"),
        doc("near", "Acme harmonic", "harmonic", "wanted"),
        doc("high-wrong", "Acme target target target", "target", "wrong"),
        doc("eligible-low", "Acme eligible", "target", "wanted"),
        doc("only-one", "Acme alpha", "neutral", "wanted"),
        doc("boost-title", "Acme zorb", "neutral", "wanted"),
        doc("boost-body", "Acme neutral", "zorb", "wanted"),
        doc("short", "Acme İx", "foo\u{2003}bar baz", "wanted"),
        doc("the-only", "Acme the", "neutral", "wanted"),
    ]);
    let backend = tantivy(&qidx, SearchConfig::default());
    for (label, params) in [
        ("and_split_fields", SearchParams { query: "alpha beta".into(), ..Default::default() }),
        ("mixed_phrase_and_term", SearchParams { query: "\"circle of fifths\" cadence".into(), query_mode: Some(QueryMode::And), ..Default::default() }),
        ("quoted_stopword_phrase_original_doc", SearchParams { query: "\"circle of fifths\"".into(), ..Default::default() }),
        ("quoted_stopword_phrase_adjacent_doc", SearchParams { query: "\"circle of fifths\"".into(), ..Default::default() }),
        ("minmatch_one_of_three", SearchParams { query: "alpha beta gamma".into(), query_mode: Some(QueryMode::MinimumMatch), ..Default::default() }),
    ] {
        let results = backend.search(params).await.unwrap(); emit(label, ids(&results));
    }
    let quote = backend.search(SearchParams { query: "\"circle of fifths\"".into(), ..Default::default() }).await.unwrap();
    emit("quote_vs_adjacency_actual_ids", ids(&quote));
    let no_stopwords = tantivy(&qidx, SearchConfig { stopwords_enabled: false, ..Default::default() });
    let quote_exact = no_stopwords.search(SearchParams { query: "\"circle of fifths\"".into(), ..Default::default() }).await.unwrap();
    emit("quoted_phrase_stopwords_disabled_control", ids(&quote_exact));
    let mixed = backend.search(SearchParams { query: "\"circle of fifths\" cadence".into(), query_mode: Some(QueryMode::And), ..Default::default() }).await.unwrap();
    emit("mixed_phrase_actual_ids", ids(&mixed));
    let mode_and = backend.search(SearchParams { query: "alpha beta".into(), query_mode: Some(QueryMode::And), ..Default::default() }).await.unwrap();
    let mode_or = backend.search(SearchParams { query: "alpha beta".into(), query_mode: Some(QueryMode::Or), ..Default::default() }).await.unwrap();
    emit("request_mode_same_query_and", ids(&mode_and)); emit("request_mode_same_query_or", ids(&mode_or));
    let m100 = tantivy(&qidx, SearchConfig { query_mode: QueryMode::MinimumMatch, minimum_match_percent: 1.0, ..Default::default() });
    let minresult = m100.search(SearchParams { query: "alpha beta gamma".into(), ..Default::default() }).await.unwrap(); emit("minimum_match_100pct_one_term", ids(&minresult));
    let boost = tantivy(&qidx, SearchConfig { field_boost_title: 0.01, field_boost_description: 100.0, field_boost_content: 100.0, ..Default::default() });
    let boosted = boost.search(SearchParams { query: "zorb".into(), ..Default::default() }).await.unwrap(); emit("configured_reverse_boost_ranking", format!("{}|{}", ids(&boosted), boosted.items.first().map(|r| format!("{}:{:.3}", r.id, r.relevance)).unwrap_or_default()));
    let filtered1 = backend.search(SearchParams { query: "target".into(), category: Some("wanted".into()), limit: Some(1), ..Default::default() }).await.unwrap();
    let filteredall = backend.search(SearchParams { query: "target".into(), category: Some("wanted".into()), limit: Some(20), ..Default::default() }).await.unwrap();
    let unfiltered = backend.search(SearchParams { query: "target".into(), limit: Some(20), ..Default::default() }).await.unwrap();
    emit("limit1_filtered", format!("total={};items={}", filtered1.total, ids(&filtered1)));
    emit("limit20_filtered", format!("total={};items={}", filteredall.total, ids(&filteredall)));
    emit("limit20_unfiltered", format!("total={};items={};first={}", unfiltered.total, ids(&unfiltered), unfiltered.items.first().map(|r| format!("{}:{:.3}", r.id, r.relevance)).unwrap_or_default()));
    let simple_dir = qtmp.path().join("simple"); write(&simple_dir.join("one.md"), "---\ntitle: Acme needle\ncategory: wanted\n---\nThis is a long sequence around needle and afterward.\n");
    let simple_cfg = SearchConfig { backend: "simple".into(), content_path: Some(simple_dir.to_string_lossy().into_owned()), snippet_length: 40, ..Default::default() };
    let simple = SimpleSearch::with_default_extractor(&simple_cfg);
    let simple_snip = simple.search(SearchParams { query: "needle".into(), snippet_length: Some(5), ..Default::default() }).await.unwrap();
    let simple_snip_long = simple.search(SearchParams { query: "needle".into(), snippet_length: Some(40), ..Default::default() }).await.unwrap();
    emit("simple_request_snippet_length", format!("short={:?};long={:?}", simple_snip.items[0].snippet, simple_snip_long.items[0].snippet));
    let short_snip_backend = tantivy(&qidx, SearchConfig { snippet_length: 4, ..Default::default() });
    let snip_task = tokio::spawn(async move { short_snip_backend.search(SearchParams { query: "baz".into(), snippet_length: Some(100), ..Default::default() }).await });
    emit("tantivy_emspace_snippet_panics", snip_task.await.is_err());
    let short_cfg_backend = tantivy(&qidx, SearchConfig { snippet_length: 12, ..Default::default() });
    let snip_a = short_cfg_backend.search(SearchParams { query: "target".into(), snippet_length: Some(1), ..Default::default() }).await.unwrap();
    let snip_b = short_cfg_backend.search(SearchParams { query: "target".into(), snippet_length: Some(100), ..Default::default() }).await.unwrap();
    emit("tantivy_request_snippet_length_ignored", format!("equal={};a={:?};b={:?}", snip_a.items.first().and_then(|r| r.snippet.as_ref())==snip_b.items.first().and_then(|r| r.snippet.as_ref()), snip_a.items.first().and_then(|r| r.snippet.as_ref()), snip_b.items.first().and_then(|r| r.snippet.as_ref())));
    let zero = tokio::spawn(async move { backend.search(SearchParams { query: "target".into(), limit: Some(0), ..Default::default() }).await });
    emit("tantivy_zero_limit_task_panics", zero.await.is_err());
    let backend = tantivy(&qidx, SearchConfig::default());

    let fuzzy = tantivy(&qidx, SearchConfig { fuzzy_enabled: true, fuzzy_distance: 1, stopwords_enabled: false, ..Default::default() });
    let exact = backend.search(SearchParams { query: "zorbq".into(), ..Default::default() }).await.unwrap();
    let fuzzyres = fuzzy.search(SearchParams { query: "zorbq".into(), ..Default::default() }).await.unwrap();
    emit("default_fuzzy_off_typo_ids", ids(&exact)); emit("configured_fuzzy_typo_ids", ids(&fuzzyres));
    let stop = backend.search(SearchParams { query: "what is a cadence".into(), ..Default::default() }).await.unwrap(); emit("stopword_positive", ids(&stop));
    let allstops = backend.search(SearchParams { query: "the a an".into(), ..Default::default() }).await.unwrap(); emit("all_stopwords_fallback_actual_ids", ids(&allstops));
    let shifted_idx = qtmp.path().join("shifted-index");
    index_docs(&shifted_idx, &[doc("shifted", "Acme", "İ0123456789-needle ABCDEFGHIJ", "wanted"), doc("ascii-shifted", "Acme", "0123456789-needle ABCDEFGHIJ", "wanted")]);
    let shifted_backend = tantivy(&shifted_idx, SearchConfig { snippet_length: 8, ..Default::default() });
    let shifted_results = shifted_backend.search(SearchParams { query: "needle".into(), ..Default::default() }).await.unwrap();
    emit("tantivy_shifted_excerpt_controls", shifted_results.items.iter().map(|r| format!("{}={:?}", r.id, r.snippet)).collect::<Vec<_>>().join(";"));
    let long_index = qtmp.path().join("snippet-index");
    index_docs(&long_index, &[doc("long-snippet", "Acme note", "start alpha beta gamma delta epsilon target zeta eta theta end", "wanted")]);
    let long_backend = tantivy(&long_index, SearchConfig { snippet_length: 12, ..Default::default() });
    let req_short = long_backend.search(SearchParams { query: "target".into(), snippet_length: Some(1), ..Default::default() }).await.unwrap();
    let req_long = long_backend.search(SearchParams { query: "target".into(), snippet_length: Some(100), ..Default::default() }).await.unwrap();
    emit("tantivy_request_snippet_length_ignored_long_control", format!("equal={};a={:?};b={:?}", req_short.items[0].snippet==req_long.items[0].snippet, req_short.items[0].snippet, req_long.items[0].snippet));
    let fallback_config = SearchConfig { backend: "tantivy".into(), content_path: Some(simple_dir.to_string_lossy().into_owned()), index_path: Some(qtmp.path().join("absent-index").to_string_lossy().into_owned()), ..Default::default() };
    let fallback = create_search_backend(&fallback_config).await.unwrap();
    let fallbackres = fallback.search(SearchParams { query: "needle".into(), ..Default::default() }).await.unwrap(); emit("missing_tantivy_index_backend_result", format!("{}|{}", fallback.name(), ids(&fallbackres)));

    // Same input through public fallback/indexed paths; the MCP consumer forwards custom fields.
    let field_dir = qtmp.path().join("field-search");
    let field_content = "---\ntitle: Acme\ncategory: categoryneedle\nsource: sourceneedle\ntier: basic\ntags: [tagneedle]\n---\nneutral body\n";
    write(&field_dir.join("field.md"), field_content);
    let extracted_field = ConceptCardDocumentExtractor::new().extract(&field_dir.join("field.md"), field_content).unwrap();
    let field_idx = qtmp.path().join("field-index"); index_docs(&field_idx, &[extracted_field]);
    let field_tantivy = tantivy(&field_idx, SearchConfig::default());
    let field_simple = SimpleSearch::with_default_extractor(&SearchConfig { backend: "simple".into(), content_path: Some(field_dir.to_string_lossy().into_owned()), ..Default::default() });
    for term in ["categoryneedle", "sourceneedle", "tagneedle"] {
        let a = field_simple.search(SearchParams { query: term.into(), ..Default::default() }).await.unwrap();
        let b = field_tantivy.search(SearchParams { query: term.into(), ..Default::default() }).await.unwrap();
        emit(&format!("simple_tantivy_field_{term}"), format!("simple={};tantivy={}", ids(&a), ids(&b)));
    }
    let extra = SearchParams { query: "*".into(), extra_filters: Some(serde_json::json!({"tier":"advanced"})), ..Default::default() };
    let simple_extra = field_simple.search(extra.clone()).await.unwrap();
    let tantivy_extra = field_tantivy.search(extra).await.unwrap();
    emit("custom_tier_filter_request", format!("simple={};tantivy={}", ids(&simple_extra), ids(&tantivy_extra)));
    let unified = Path::new("/data/unified/concept.md");
    let concepts_unified = Path::new("/data/concepts-unified/concept.md");
    emit("content_type_unified_directory_conventions", format!("metadata_unified={:?};fts_unified={:?};metadata_concepts_unified={:?};fts_concepts_unified={:?}", fabryk_content::metadata::detect_content_type(unified), ConceptCardDocumentExtractor::new().extract(unified, "# Acme\n").unwrap().content_type, fabryk_content::metadata::detect_content_type(concepts_unified), ConceptCardDocumentExtractor::new().extract(concepts_unified, "# Acme\n").unwrap().content_type));

    // 6. Build/append/rebuild freshness and actual searchable generations.
    let itmp = tempfile::tempdir().unwrap(); let a = itmp.path().join("A"); let b = itmp.path().join("B"); let idx = itmp.path().join("idx");
    write(&a.join("a.md"), "Acme alphaone"); write(&b.join("b.md"), "Acme betatwo");
    let builder = IndexBuilder::new().force_rebuild(); builder.build(&a, &idx).await.unwrap();
    IndexBuilder::new().force_rebuild().build_append(&b, &idx).await.unwrap();
    let ids_before = ids(&tantivy(&idx, SearchConfig::default()).search(SearchParams { query: "*".into(), limit: Some(20), ..Default::default() }).await.unwrap()); emit("A_build_B_append_ids", ids_before);
    IndexBuilder::new().force_rebuild().build(&a, &idx).await.unwrap();
    let after_rebuild = IndexBuilder::new().build_append(&b, &idx).await.unwrap();
    let after_ids = ids(&tantivy(&idx, SearchConfig::default()).search(SearchParams { query: "*".into(), limit: Some(20), ..Default::default() }).await.unwrap()); emit("A_force_rebuild_B_unchanged_append", format!("processed={};indexed={};ids={after_ids}", after_rebuild.files_processed, after_rebuild.documents_indexed));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write(&b.join("b.md"), "Acme betanew");
    let update = IndexBuilder::new().build_append(&b, &idx).await.unwrap();
    let update_ids = ids(&tantivy(&idx, SearchConfig::default()).search(SearchParams { query: "*".into(), limit: Some(20), ..Default::default() }).await.unwrap()); emit("append_changed_same_id", format!("processed={};indexed={};ids={update_ids}", update.files_processed, update.documents_indexed));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::remove_file(b.join("b.md")).unwrap();
    let deletion = IndexBuilder::new().build_append(&b, &idx).await.unwrap();
    let deletion_ids = ids(&tantivy(&idx, SearchConfig::default()).search(SearchParams { query: "*".into(), limit: Some(20), ..Default::default() }).await.unwrap()); emit("append_source_deletion", format!("processed={};indexed={};ids={deletion_ids}", deletion.files_processed, deletion.documents_indexed));

    let duptmp = tempfile::tempdir().unwrap(); let da = duptmp.path().join("a"); let db = duptmp.path().join("b"); let didx = duptmp.path().join("idx");
    write(&da.join("shared.md"), "Acme commonword"); write(&db.join("shared.md"), "Acme commonword");
    IndexBuilder::new().force_rebuild().build(&da, &didx).await.unwrap();
    IndexBuilder::new().force_rebuild().build_append(&db, &didx).await.unwrap();
    let dupids = tantivy(&didx, SearchConfig::default()).search(SearchParams { query: "*".into(), limit: Some(20), ..Default::default() }).await.unwrap();
    emit("same_id_across_appended_roots", format!("total={};ids={}", dupids.total, ids(&dupids)));

    let same = tempfile::tempdir().unwrap(); let content = same.path().join("docs"); let sidx = same.path().join("idx");
    let tracked = content.join("same.md"); write(&tracked, "Acme oldneedle");
    IndexBuilder::new().build(&content, &sidx).await.unwrap();
    let before_time = fs::metadata(&tracked).unwrap().modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap();
    write(&tracked, "Acme newneedle");
    let after_time = fs::metadata(&tracked).unwrap().modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap();
    let mtime = format!("before={}.{:09};after={}.{:09}", before_time.as_secs(), before_time.subsec_nanos(), after_time.as_secs(), after_time.subsec_nanos());
    emit("controlled_mtime_observation", format!("{mtime};same_integer_seconds={}", before_time.as_secs()==after_time.as_secs()));
    let freshness = IndexBuilder::new().build(&content, &sidx).await.unwrap();
    let sb = tantivy(&sidx, SearchConfig::default());
    let old = ids(&sb.search(SearchParams { query: "oldneedle".into(), ..Default::default() }).await.unwrap());
    let new = ids(&sb.search(SearchParams { query: "newneedle".into(), ..Default::default() }).await.unwrap());
    emit("same_second_freshness_rebuild_stats", format!("processed={};indexed={};old={old};new={new}", freshness.files_processed, freshness.documents_indexed));

    // 7. Isolated SimpleSearch and source-validator contract controls.
    let empty_config = SearchConfig::default(); let no_content = SimpleSearch::with_default_extractor(&empty_config);
    emit("simple_no_content_path", format!("{}", no_content.search(SearchParams { query: "*".into(), ..Default::default() }).await.unwrap().total));
    emit("slice03_probe_finished", true);
}
