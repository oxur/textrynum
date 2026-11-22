use std::collections::HashMap;
use std::fs::{self, File, FileTimes};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};
use fabryk_content::markdown::parser::extract_first_paragraph;
use fabryk_content::metadata::{ContentType, extract_metadata};
use fabryk_content::sources::{SourceCategory, SourceResolver, ValidationMode, validate_sources};
use fabryk_fts::{ConceptCardDocumentExtractor, IndexBuilder, Indexer, SearchBackend, SearchConfig, SearchDocument, SearchParams, SearchResults, SearchSchema, SimpleSearch, TantivySearch};
use fabryk_fts::types::QueryMode;

fn emit(key: &str, value: impl std::fmt::Debug) { println!("{key}={value:?}"); }
fn doc(id: &str, title: &str, content: &str, category: &str) -> SearchDocument {
    SearchDocument::builder().id(id).title(title).content(content).category(category).build()
}
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap(); fs::write(path, text).unwrap();
}
fn index(path: &Path, docs: &[SearchDocument]) {
    let schema=SearchSchema::build(); let mut writer=Indexer::new(path,&schema).unwrap();
    for item in docs { writer.add_document(item).unwrap(); }
    writer.commit().unwrap();
}
fn engine(path: &Path, mut config: SearchConfig) -> TantivySearch {
    config.index_path=Some(path.to_string_lossy().into_owned()); TantivySearch::new(&config).unwrap()
}
fn ids(r: &SearchResults) -> Vec<String> { let mut a=r.items.iter().map(|x|x.id.clone()).collect::<Vec<_>>();a.sort();a }
fn names(a: &[&str]) -> Vec<String> { let mut a=a.iter().map(|s|s.to_string()).collect::<Vec<_>>();a.sort();a }
async fn all_ids(path: &Path) -> Vec<String> {
    ids(&engine(path,SearchConfig::default()).search(SearchParams{query:"*".into(),limit:Some(50),..Default::default()}).await.unwrap())
}
fn simple(path: &Path, snippet_length: usize) -> SimpleSearch {
    SimpleSearch::with_default_extractor(&SearchConfig{backend:"simple".into(),content_path:Some(path.to_string_lossy().into_owned()),snippet_length,..Default::default()})
}

#[tokio::main]
async fn main() {
    // Assertions establish current diagnosis, not repaired behavior. All fixtures are owned/local.
    assert!(catch_unwind(||extract_first_paragraph("# Acme\n\néxample body",1)).is_err());
    let guide_limit_text=format!("# Acme\n\n{}é tail","a".repeat(199));
    assert!(catch_unwind(||extract_first_paragraph(&guide_limit_text,200)).is_err());
    assert_eq!(extract_first_paragraph("# Acme\n\nalpha beta",5).unwrap(),"alpha...");
    assert_eq!(extract_first_paragraph("alpha",0).unwrap(),"...");
    assert_eq!(extract_first_paragraph("alpha",5).unwrap(),"alpha");
    emit("FA011_public_helper_limit1_and_actual_consumer_limit200_panics_with_ASCII_zero_exact_controls",true);

    let snip=doc("snippet","Acme","foo\u{2003}bar baz","general");
    assert!(catch_unwind(AssertUnwindSafe(||snip.extract_snippet("baz",4))).is_err());
    let st=tempfile::tempdir().unwrap();write(&st.path().join("unicode.md"),"# Acme\n\nfoo\u{2003}bar baz");
    let ss=simple(st.path(),4);
    let simple_panic=tokio::spawn(async move{ss.search(SearchParams{query:"baz".into(),..Default::default()}).await}).await.unwrap_err();
    assert!(simple_panic.is_panic());
    let ti=st.path().join("index");index(&ti,&[snip]);let ts=engine(&ti,SearchConfig{snippet_length:4,..Default::default()});
    assert!(tokio::spawn(async move{ts.search(SearchParams{query:"baz".into(),..Default::default()}).await}).await.unwrap_err().is_panic());
    emit("FA012_helper_actual_Simple_and_actual_Tantivy_multibyte_whitespace_panics",true);
    let shifted=doc("shifted","Acme","İ0123456789needleABCDEFGHIJ","general");
    let ascii=doc("ascii","Acme","0123456789needleABCDEFGHIJ","general");
    let pair=(shifted.extract_snippet("needle",8).unwrap(),ascii.extract_snippet("needle",8).unwrap());
    assert_eq!(pair,("...9needleA...".to_string(),"...89needle...".to_string()));emit("FA013_helper_case_expansion_pair",&pair);
    let ci=st.path().join("case-index");index(&ci,&[doc("shifted","Acme","İ0123456789-needle ABCDEFGHIJ","general"),doc("ascii","Acme","0123456789-needle ABCDEFGHIJ","general")]);
    let r=engine(&ci,SearchConfig{snippet_length:8,..Default::default()}).search(SearchParams{query:"needle".into(),..Default::default()}).await.unwrap();
    let excerpts=r.items.iter().map(|x|(x.id.as_str(),x.snippet.as_deref().unwrap())).collect::<HashMap<_,_>>();
    assert_eq!(excerpts["shifted"],"...-needle...");assert_eq!(excerpts["ascii"],"...9-needle...");emit("FA013_actual_Tantivy_case_expansion_pair",&excerpts);

    let mt=tempfile::tempdir().unwrap();let cards=mt.path().join("cards");let path=cards.join("card.md");
    let text="---\nchapter: Chapter fallback\nsource: Acme Inc.\n---\n# Body heading\n\nBody text.";write(&path,text);
    let wrong=extract_metadata(&cards,&path,ContentType::ConceptCard).await.unwrap();
    let body_control=ConceptCardDocumentExtractor::new().extract(&path,text).unwrap();assert_eq!(body_control.title,"Body heading");
    assert!(wrong.title.contains("chapter: Chapter fallback"));assert_ne!(wrong.title,"Body heading");
    let chapter=extract_metadata(&cards,&path,ContentType::SourceChapter).await.unwrap();assert_eq!(chapter.title,"Chapter fallback");
    write(&path,"---\ntitle: Explicit Acme\n---\n# Body heading\n");assert_eq!(extract_metadata(&cards,&path,ContentType::ConceptCard).await.unwrap().title,"Explicit Acme");
    emit("FA015_own_metadata_wrong_title_with_explicit_title_and_chapter_controls",wrong.title);
    let sources=mt.path().join("sources");fs::create_dir_all(&sources).unwrap();let base=sources.to_string_lossy().into_owned();
    let collision=HashMap::from([("a-b".into(),SourceCategory{path:base.clone(),files:HashMap::from([("c".into(),"Acme One.pdf".into())]),aliases:HashMap::new()}),("a".into(),SourceCategory{path:base.clone(),files:HashMap::from([("b-c".into(),"Acme Two.pdf".into())]),aliases:HashMap::new()})]);
    let resolver=SourceResolver::from_categories(&collision);assert_eq!(resolver.known_ids().len(),1);assert!(resolver.known_ids().contains("a-b-c"));
    let empty_cards=mt.path().join("empty-cards");fs::create_dir_all(&empty_cards).unwrap();let validation_cards=empty_cards.clone();
    assert!(tokio::spawn(async move{validate_sources(&validation_cards,&collision,ValidationMode::ConfigFilesystem,false,0.6).await}).await.unwrap_err().is_panic());
    emit("FA017_colliding_IDs_collapse_and_actual_debug_validator_count_underflows",true);
    fs::create_dir_all(sources.join("directory.pdf")).unwrap();write(&sources.join("file.pdf"),"Acme fixture");
    let mut outcomes=Vec::new();
    for filename in ["file.pdf","directory.pdf","missing.pdf"] {
        let categories=HashMap::from([("general".into(),SourceCategory{path:base.clone(),files:HashMap::from([("one".into(),filename.into())]),aliases:HashMap::new()})]);
        let r=validate_sources(&empty_cards,&categories,ValidationMode::ConfigFilesystem,false,0.6).await.unwrap();outcomes.push((filename,r.is_valid(),r.stats.sources_on_disk));
    }
    assert_eq!(outcomes,vec![("file.pdf",true,1),("directory.pdf",true,1),("missing.pdf",false,0)]);emit("FA019_file_directory_missing_controls",outcomes);

    let qt=tempfile::tempdir().unwrap();let qi=qt.path().join("index");index(&qi,&[
        doc("split","Acme alpha","beta body","wanted"),doc("one","Acme alpha","neutral","wanted"),doc("both","Acme alpha beta","neutral","wanted"),
        doc("phrase","Acme circle of fifths","circle of fifths","wanted"),doc("adjacent","Acme circle fifths","circle fifths","wanted"),doc("cadence","Acme cadence","cadence","wanted"),
        doc("high-wrong","Acme target target target","target","wrong"),doc("eligible-low","Acme eligible","target","wanted"),
        doc("boost-title","Acme zorb","neutral","wanted"),doc("boost-body","Acme neutral","zorb","wanted")]);
    let default=engine(&qi,SearchConfig::default());
    let quote=default.search(SearchParams{query:"\"circle of fifths\"".into(),..Default::default()}).await.unwrap();
    let no_stop=engine(&qi,SearchConfig{stopwords_enabled:false,..Default::default()});
    let exact=no_stop.search(SearchParams{query:"\"circle of fifths\"".into(),..Default::default()}).await.unwrap();
    assert_eq!(ids(&quote),names(&["adjacent"]));assert_eq!(ids(&exact),names(&["phrase"]));emit("FA020_quoted_stopword_enabled_vs_disabled",(ids(&quote),ids(&exact)));
    let and=engine(&qi,SearchConfig{query_mode:QueryMode::And,stopwords_enabled:false,..Default::default()});
    let or=engine(&qi,SearchConfig{query_mode:QueryMode::Or,stopwords_enabled:false,..Default::default()});
    let and_result=and.search(SearchParams{query:"alpha beta".into(),..Default::default()}).await.unwrap();
    let or_result=or.search(SearchParams{query:"alpha beta".into(),..Default::default()}).await.unwrap();
    assert_eq!(ids(&and_result),names(&["both"]));assert_eq!(ids(&or_result),names(&["split","one","both"]));
    let mixed=and.search(SearchParams{query:"\"absent phrase\" cadence".into(),..Default::default()}).await.unwrap();assert_eq!(ids(&mixed),names(&["cadence"]));
    assert!(and.search(SearchParams{query:"\"absent phrase\"".into(),..Default::default()}).await.unwrap().items.is_empty());
    emit("FA021_CONFIGURED_And_split_fields_and_absent_phrase_with_Or_controls",(ids(&and_result),ids(&or_result),ids(&mixed)));
    let override_or=and.search(SearchParams{query:"alpha beta".into(),query_mode:Some(QueryMode::Or),..Default::default()}).await.unwrap();
    assert_eq!(ids(&override_or),ids(&and_result));assert_ne!(ids(&override_or),ids(&or_result));emit("FA023_request_Or_ignored_with_actual_configured_Or_counterexample",ids(&override_or));
    let min=engine(&qi,SearchConfig{query_mode:QueryMode::MinimumMatch,minimum_match_percent:1.0,stopwords_enabled:false,..Default::default()});
    let min_result=min.search(SearchParams{query:"alpha beta gamma".into(),..Default::default()}).await.unwrap();assert_eq!(ids(&min_result),names(&["one","split","both"]));emit("FA022_CONFIGURED_100pct_returns_partial_matches",ids(&min_result));
    let base_rank=default.search(SearchParams{query:"zorb".into(),..Default::default()}).await.unwrap();
    let reversed=engine(&qi,SearchConfig{field_boost_title:0.01,field_boost_description:100.0,field_boost_content:100.0,..Default::default()}).search(SearchParams{query:"zorb".into(),..Default::default()}).await.unwrap();
    assert_eq!(base_rank.items[0].id,"boost-title");assert_eq!(reversed.items[0].id,"boost-title");emit("FA024_reversed_boosts_do_not_reverse_actual_ranking",true);
    let all=default.search(SearchParams{query:"target".into(),limit:Some(20),..Default::default()}).await.unwrap();assert_eq!(all.items[0].id,"high-wrong");
    let small=default.search(SearchParams{query:"target".into(),category:Some("wanted".into()),limit:Some(1),..Default::default()}).await.unwrap();
    let large=default.search(SearchParams{query:"target".into(),category:Some("wanted".into()),limit:Some(20),..Default::default()}).await.unwrap();
    assert!(small.items.is_empty());assert_eq!(small.total,1);assert_eq!(ids(&large),names(&["eligible-low"]));assert_eq!(large.total,2);emit("FA025_filtered_small_large_unfiltered_counts",(small.total,ids(&small),large.total,ids(&large),all.total));
    let zero=engine(&qi,SearchConfig::default());assert!(tokio::spawn(async move{zero.search(SearchParams{query:"target".into(),limit:Some(0),..Default::default()}).await}).await.unwrap_err().is_panic());
    let sd=qt.path().join("simple");write(&sd.join("one.md"),"# Acme\n\nstart alpha beta gamma delta epsilon target zeta eta theta end");
    let s=simple(&sd,12);let zero_simple=s.search(SearchParams{query:"target".into(),limit:Some(0),..Default::default()}).await.unwrap();assert!(zero_simple.items.is_empty());assert_eq!(zero_simple.total,1);emit("FA026_actual_Tantivy_panic_and_actual_Simple_zero_limit_empty",true);
    let si=qt.path().join("snippet-index");index(&si,&[doc("long","Acme","start alpha beta gamma delta epsilon target zeta eta theta end","wanted")]);let snippets=engine(&si,SearchConfig{snippet_length:12,..Default::default()});
    let short=snippets.search(SearchParams{query:"target".into(),snippet_length:Some(1),..Default::default()}).await.unwrap();let long=snippets.search(SearchParams{query:"target".into(),snippet_length:Some(100),..Default::default()}).await.unwrap();
    assert_eq!(short.items[0].snippet,long.items[0].snippet);assert!(short.items[0].snippet.is_some());
    let s_short=s.search(SearchParams{query:"target".into(),snippet_length:Some(1),..Default::default()}).await.unwrap();let s_long=s.search(SearchParams{query:"target".into(),snippet_length:Some(100),..Default::default()}).await.unwrap();assert_ne!(s_short.items[0].snippet,s_long.items[0].snippet);emit("FA027_request_1_100_Tantivy_equal_Simple_distinct",(&short.items[0].snippet,&s_short.items[0].snippet,&s_long.items[0].snippet));

    let it=tempfile::tempdir().unwrap();let a=it.path().join("A");let b=it.path().join("B");let ii=it.path().join("index");write(&a.join("a.md"),"Acme alphaone");write(&b.join("b.md"),"Acme betatwo");
    IndexBuilder::new().force_rebuild().build(&a,&ii).await.unwrap();IndexBuilder::new().force_rebuild().build_append(&b,&ii).await.unwrap();assert_eq!(all_ids(&ii).await,names(&["a","b"]));
    IndexBuilder::new().force_rebuild().build(&a,&ii).await.unwrap();let skipped=IndexBuilder::new().build_append(&b,&ii).await.unwrap();assert_eq!(skipped.files_processed,0);assert_eq!(all_ids(&ii).await,names(&["a"]));
    IndexBuilder::new().force_rebuild().build_append(&b,&ii).await.unwrap();assert_eq!(all_ids(&ii).await,names(&["a","b"]));emit("FA030_rebuild_stale_append_cache_omits_B_forced_append_restores",true);
    let da=it.path().join("duplicate-A");let db=it.path().join("duplicate-B");let di=it.path().join("duplicate-index");write(&da.join("shared.md"),"Acme first");write(&db.join("shared.md"),"Acme second");
    IndexBuilder::new().force_rebuild().build(&da,&di).await.unwrap();IndexBuilder::new().force_rebuild().build_append(&db,&di).await.unwrap();assert_eq!(all_ids(&di).await,names(&["shared","shared"]));emit("FA032_actual_multi_root_duplicate_IDs",true);
    let content=it.path().join("timed-docs");let timed=content.join("same.md");let mi=it.path().join("mtime-index");write(&timed,"Acme oldneedle");
    let t1=UNIX_EPOCH+Duration::new(1_700_000_000,100_000_000);let t2=UNIX_EPOCH+Duration::new(1_700_000_000,900_000_000);
    File::options().write(true).open(&timed).unwrap().set_times(FileTimes::new().set_modified(t1)).unwrap();let observed1=fs::metadata(&timed).unwrap().modified().unwrap().duration_since(UNIX_EPOCH).unwrap();
    IndexBuilder::new().build(&content,&mi).await.unwrap();write(&timed,"Acme newneedle");File::options().write(true).open(&timed).unwrap().set_times(FileTimes::new().set_modified(t2)).unwrap();
    let observed2=fs::metadata(&timed).unwrap().modified().unwrap().duration_since(UNIX_EPOCH).unwrap();assert_eq!(observed1.as_secs(),observed2.as_secs());assert_ne!(observed1.subsec_nanos(),observed2.subsec_nanos());
    let stale=IndexBuilder::new().build(&content,&mi).await.unwrap();assert_eq!(stale.files_processed,0);
    let e=engine(&mi,SearchConfig::default());let old=e.search(SearchParams{query:"oldneedle".into(),..Default::default()}).await.unwrap();let new=e.search(SearchParams{query:"newneedle".into(),..Default::default()}).await.unwrap();assert_eq!(ids(&old),names(&["same"]));assert!(new.items.is_empty());
    emit("FA033_CONTROLLED_same_second_distinct_nanos_stale_old_term",(observed1,observed2,ids(&old),ids(&new)));
    emit("ALL_CDC_LOCAL_ASSERTIONS_COMPLETED_NOT_REPAIR_ACCEPTANCE",true);
}
