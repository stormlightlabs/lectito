use lectito_core::{PreprocessConfig, preprocess_html};
use std::fs;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from("../../tests/fixtures/preprocess")
}

fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixture_root().join(path)).unwrap()
}

#[test]
fn rollingstone_fixture_resolves_lazy_images_and_noscript() {
    let html = read_fixture("lazy_images/rollingstone-best-albums.html");
    let result = preprocess_html(&html, &PreprocessConfig::default());

    assert!(!result.contains("lazyload-fallback.gif"));
    assert!(!result.contains("<noscript"));
    assert!(result.contains(
        "src=\"https://www.rollingstone.com/wp-content/uploads/2020/09/R1344-500-Arcade-Fire-Funeral.jpg?w=300\""
    ));
    assert!(result.contains("srcset=\"https://www.rollingstone.com/wp-content/uploads/2020/09/R1344-500-Arcade-Fire-Funeral.jpg 1000w, https://www.rollingstone.com/wp-content/uploads/2020/09/R1344-500-Arcade-Fire-Funeral.jpg?resize=150,150 150w, https://www.rollingstone.com/wp-content/uploads/2020/09/R1344-500-Arcade-Fire-Funeral.jpg?resize=300,300 300w\""));
}

#[test]
fn norvig_fixture_normalizes_font_tags_from_real_markup() {
    let html = read_fixture("phrasing/norvig-21-days.html");
    let result = preprocess_html(&html, &PreprocessConfig::default());

    assert!(!result.contains("<font"));
    assert!(result.contains("<span face=\"modern\">"));
    assert!(result.contains("<div style=\"max-width: 52em\">"));
}

#[test]
fn mit_tao_fixture_normalizes_legacy_font_markup() {
    let html = read_fixture("phrasing/mit-tao.html");
    let result = preprocess_html(&html, &PreprocessConfig::default());

    assert!(!result.contains("<font"));
    assert!(result.contains("<span size=\"+1\"><b>"));
    assert!(result.contains("<br>---Alex"));
}

#[test]
fn video_embed_fixture_preserves_allowed_players_and_drops_other_iframes() {
    let fixtures = [
        (
            "video_embeds/musicradar-guitar-solos.html",
            "youtube.com/embed/solo-lesson",
            "ads.example.com/pre-roll",
        ),
        (
            "video_embeds/vulture-best-movies-2025.html",
            "player.vimeo.com/video/2025",
            "tracker.example.com/embed",
        ),
        (
            "video_embeds/polygon-zelda-review.html",
            "upload.wikimedia.org/gameplay/totk-review.webm",
            "sponsor.example.com/player",
        ),
        (
            "video_embeds/cloudflare-workers-ai.html",
            "player.twitch.tv/?video=42",
            "analytics.example.com/embed",
        ),
    ];

    for (path, expected, rejected) in fixtures {
        let html = read_fixture(path);
        let result = preprocess_html(&html, &PreprocessConfig::default());

        assert!(result.contains(expected), "{path} should preserve allowed media");
        assert!(!result.contains(rejected), "{path} should drop disallowed embeds");
    }
}
