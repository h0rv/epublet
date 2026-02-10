use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mu_epub::{EpubBook, MemoryBudget, RenderPrepOptions};
use mu_epub_render::{
    CancelToken, OverlayComposer, OverlayContent, OverlayItem, OverlaySize, OverlaySlot,
    PageChromeConfig, PaginationProfileId, RenderCacheStore, RenderConfig, RenderDiagnostic,
    RenderEngine, RenderEngineError, RenderEngineOptions, RenderPage,
};

fn fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(
        "../../tests/fixtures/Fundamental-Accessibility-Tests-Basic-Functionality-v2.0.0.epub",
    );
    path
}

fn open_fixture_book() -> EpubBook<std::fs::File> {
    EpubBook::open(fixture_path()).expect("fixture EPUB should open")
}

fn build_engine() -> RenderEngine {
    let mut opts = RenderEngineOptions::for_display(420, 180);
    opts.layout.page_chrome = PageChromeConfig {
        progress_enabled: true,
        footer_enabled: true,
        ..PageChromeConfig::default()
    };
    RenderEngine::new(opts)
}

fn chapter_with_min_pages(
    engine: &RenderEngine,
    book: &mut EpubBook<std::fs::File>,
    min_pages: usize,
) -> Option<(usize, Vec<RenderPage>)> {
    for chapter in 0..book.chapter_count() {
        let pages = engine
            .prepare_chapter(book, chapter)
            .expect("full chapter render should succeed");
        if pages.len() >= min_pages {
            return Some((chapter, pages));
        }
    }
    None
}

#[test]
fn prepare_chapter_page_range_matches_full_slice() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let (chapter, full) = chapter_with_min_pages(&engine, &mut book, 3)
        .expect("fixture should contain a chapter with at least 3 pages");

    let start = 1usize;
    let end = (start + 2).min(full.len());
    let expected = full[start..end].to_vec();

    let actual = engine
        .prepare_chapter_page_range(&mut book, chapter, start, end)
        .expect("range render should succeed");
    assert_eq!(actual, expected);
}

#[test]
fn prepare_chapter_iter_matches_full_render() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let chapter = 0usize;

    let full = engine
        .prepare_chapter(&mut book, chapter)
        .expect("full chapter render should succeed");
    let iterated: Vec<RenderPage> = engine
        .prepare_chapter_iter(&mut book, chapter)
        .expect("iterator render should succeed")
        .collect();

    assert_eq!(iterated, full);
}

#[test]
fn prepare_chapter_iter_streaming_matches_full_render() {
    let engine = build_engine();
    let mut book_for_full = open_fixture_book();
    let chapter = 0usize;

    let full = engine
        .prepare_chapter(&mut book_for_full, chapter)
        .expect("full chapter render should succeed");

    let streaming: Vec<RenderPage> = engine
        .prepare_chapter_iter_streaming(open_fixture_book(), chapter)
        .collect::<Result<Vec<_>, _>>()
        .expect("streaming iterator should succeed");

    assert_eq!(streaming, full);
}

#[test]
fn prepare_chapter_bytes_with_matches_full_render() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let chapter = 0usize;

    let expected = engine
        .prepare_chapter(&mut book, chapter)
        .expect("full chapter render should succeed");

    let chapter_href = book.chapter(chapter).expect("chapter should exist").href;
    let mut chapter_buf = Vec::with_capacity(128 * 1024);
    book.read_resource_into_with_hard_cap(
        &chapter_href,
        &mut chapter_buf,
        RenderPrepOptions::default().memory.max_entry_bytes,
    )
    .expect("chapter bytes should load");

    let mut actual = Vec::new();
    engine
        .prepare_chapter_bytes_with(&mut book, chapter, &chapter_buf, |page| actual.push(page))
        .expect("chapter-bytes render should succeed");

    assert_eq!(actual, expected);
}

#[test]
fn prepare_chapter_iter_streaming_reports_errors() {
    let engine = build_engine();
    let invalid_chapter = usize::MAX;
    let mut iter = engine.prepare_chapter_iter_streaming(open_fixture_book(), invalid_chapter);
    let first = iter
        .next()
        .expect("streaming iterator should produce terminal error");
    assert!(first.is_err());
    assert!(iter.next().is_none());
}

#[test]
fn pagination_profile_id_is_stable_for_same_options() {
    let e1 = build_engine();
    let e2 = build_engine();
    assert_eq!(e1.pagination_profile_id(), e2.pagination_profile_id());
}

#[derive(Clone, Copy, Debug, Default)]
struct AlreadyCancelled;

impl CancelToken for AlreadyCancelled {
    fn is_cancelled(&self) -> bool {
        true
    }
}

#[test]
fn prepare_chapter_with_cancel_stops_early() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let mut saw_pages = 0usize;
    let result =
        engine.prepare_chapter_with_cancel(&mut book, 0, &AlreadyCancelled, |_page| saw_pages += 1);
    assert!(result.is_err());
    assert_eq!(saw_pages, 0);
}

#[test]
fn prepare_chapter_with_config_can_disable_embedded_fonts() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let (chapter, _) = chapter_with_min_pages(&engine, &mut book, 1)
        .expect("fixture should contain at least one renderable chapter");
    let mut pages = Vec::new();
    engine
        .prepare_chapter_with_config(
            &mut book,
            chapter,
            RenderConfig::default().with_embedded_fonts(false),
            |page| pages.push(page),
        )
        .expect("render without embedded fonts should succeed");
    assert!(!pages.is_empty());
}

#[derive(Clone, Copy, Debug, Default)]
struct FooterOverlay;

impl OverlayComposer for FooterOverlay {
    fn compose(
        &self,
        metrics: &mu_epub_render::PageMetrics,
        _viewport: OverlaySize,
    ) -> Vec<OverlayItem> {
        vec![OverlayItem {
            slot: OverlaySlot::BottomCenter,
            z: 1,
            content: OverlayContent::Text(format!("p{}", metrics.chapter_page_index + 1)),
        }]
    }
}

#[test]
fn overlay_composer_attaches_overlay_items() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let (chapter, _) = chapter_with_min_pages(&engine, &mut book, 1)
        .expect("fixture should contain at least one renderable chapter");
    let mut pages = Vec::new();
    engine
        .prepare_chapter_with_overlay_composer(
            &mut book,
            chapter,
            OverlaySize {
                width: 420,
                height: 180,
            },
            &FooterOverlay,
            |p| pages.push(p),
        )
        .expect("overlay composer path should succeed");
    assert!(!pages.is_empty());
    assert!(pages.iter().all(|p| !p.overlay_items.is_empty()));
}

#[test]
fn diagnostic_sink_receives_reflow_timing() {
    let mut engine = build_engine();
    let seen = Arc::new(Mutex::new(Vec::<RenderDiagnostic>::new()));
    let seen_clone = Arc::clone(&seen);
    engine.set_diagnostic_sink(move |d| {
        if let Ok(mut sink) = seen_clone.lock() {
            sink.push(d);
        }
    });
    let mut book = open_fixture_book();
    let _ = engine
        .prepare_chapter(&mut book, 0)
        .expect("prepare should pass");
    let diagnostics = seen.lock().expect("diag lock").clone();
    assert!(diagnostics
        .iter()
        .any(|d| matches!(d, RenderDiagnostic::ReflowTimeMs(_))));
}

#[derive(Default)]
struct CacheSpy {
    loads: Mutex<usize>,
    stores: Mutex<usize>,
    cached_pages: Mutex<Option<Vec<RenderPage>>>,
}

impl CacheSpy {
    fn load_count(&self) -> usize {
        *self.loads.lock().expect("load lock")
    }

    fn store_count(&self) -> usize {
        *self.stores.lock().expect("store lock")
    }
}

impl RenderCacheStore for CacheSpy {
    fn load_chapter_pages(
        &self,
        _profile: PaginationProfileId,
        _chapter_index: usize,
    ) -> Option<Vec<RenderPage>> {
        let mut loads = self.loads.lock().expect("load lock");
        *loads += 1;
        self.cached_pages.lock().expect("pages lock").clone()
    }

    fn store_chapter_pages(
        &self,
        _profile: PaginationProfileId,
        _chapter_index: usize,
        pages: &[RenderPage],
    ) {
        let mut stores = self.stores.lock().expect("store lock");
        *stores += 1;
        *self.cached_pages.lock().expect("pages lock") = Some(pages.to_vec());
    }
}

#[test]
fn prepare_chapter_with_config_stores_pages_in_cache() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let cache = CacheSpy::default();
    let (chapter, _) = chapter_with_min_pages(&engine, &mut book, 1)
        .expect("fixture should contain at least one renderable chapter");

    let pages = engine
        .prepare_chapter_with_config_collect(
            &mut book,
            chapter,
            RenderConfig::default().with_cache(&cache),
        )
        .expect("prepare with cache should pass");

    assert!(!pages.is_empty());
    assert_eq!(cache.load_count(), 1);
    assert_eq!(cache.store_count(), 1);
    let cached = cache
        .cached_pages
        .lock()
        .expect("pages lock")
        .clone()
        .expect("cache should store pages");
    assert_eq!(cached, pages);
}

#[test]
fn prepare_chapter_with_config_uses_cache_hit() {
    let engine = build_engine();
    let mut book = open_fixture_book();
    let (chapter, expected) = chapter_with_min_pages(&engine, &mut book, 1)
        .expect("fixture should contain at least one renderable chapter");

    let cache = CacheSpy::default();
    *cache.cached_pages.lock().expect("pages lock") = Some(expected.clone());
    let mut book_from_cache = open_fixture_book();

    let actual = engine
        .prepare_chapter_with_config_collect(
            &mut book_from_cache,
            chapter,
            RenderConfig::default().with_cache(&cache),
        )
        .expect("cached prepare should pass");

    assert_eq!(actual, expected);
    assert_eq!(cache.load_count(), 1);
    assert_eq!(cache.store_count(), 0);
}

#[test]
fn prepare_chapter_collect_enforces_max_pages_in_memory() {
    let baseline_engine = build_engine();
    let mut baseline_book = open_fixture_book();
    let (chapter, _) = chapter_with_min_pages(&baseline_engine, &mut baseline_book, 2)
        .expect("fixture should contain a chapter with at least 2 pages");

    let mut opts = RenderEngineOptions::for_display(420, 180);
    opts.prep = RenderPrepOptions {
        memory: MemoryBudget {
            max_pages_in_memory: 1,
            ..MemoryBudget::default()
        },
        ..RenderPrepOptions::default()
    };
    let engine = RenderEngine::new(opts);
    let mut book = open_fixture_book();

    let err = engine
        .prepare_chapter(&mut book, chapter)
        .expect_err("collect path should enforce max_pages_in_memory");
    assert!(matches!(
        err,
        RenderEngineError::LimitExceeded {
            kind: "max_pages_in_memory",
            ..
        }
    ));
}
