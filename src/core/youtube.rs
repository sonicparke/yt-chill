//! YouTube scraping and parsing

use crate::error::{Result, YtChillError};
use crate::types::Video;
use regex::Regex;
use std::sync::LazyLock;
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build reqwest client")
});

static YT_INITIAL_DATA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var ytInitialData = (.+?);</script>").expect("Invalid regex"));

/// Build YouTube search URL
fn build_search_url(query: &str, filter: &str) -> String {
    let encoded_query = urlencoding::encode(query);
    let sp = match filter {
        "video" => "EgIQAQ%3D%3D",
        "channel" => "EgIQAg%3D%3D",
        _ => "",
    };
    format!(
        "https://www.youtube.com/results?search_query={}&sp={}",
        encoded_query, sp
    )
}

/// Fetch YouTube HTML with browser-like headers
async fn fetch_youtube_html(url: &str) -> Result<String> {
    let response = CLIENT
        .get(url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(YtChillError::Network(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    Ok(response.text().await?)
}

/// Extract ytInitialData JSON from YouTube HTML
fn extract_yt_initial_data(html: &str) -> Result<serde_json::Value> {
    let Some(captures) = YT_INITIAL_DATA_RE.captures(html) else {
        tracing::debug!(
            target: "yt_chill",
            "yt_initial_data_regex_miss (html_len={})",
            html.len()
        );
        return Err(YtChillError::YouTubeParse(
            "Failed to find ytInitialData".into(),
        ));
    };

    let json_str = captures.get(1).unwrap().as_str();
    serde_json::from_str(json_str).map_err(|e| {
        tracing::debug!(
            target: "yt_chill",
            "yt_initial_data_json_error error={e}"
        );
        YtChillError::YouTubeParse(format!("Failed to parse ytInitialData: {}", e))
    })
}

/// Decode HTML entities in a string
fn decode_html_entities(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}

/// First `itemSectionRenderer.contents` array on a two-column **search**
/// results page (video tab and channel tab share this outer shape).
fn search_item_section_contents(data: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    data.get("contents")?
        .get("twoColumnSearchResultsRenderer")?
        .get("primaryContents")?
        .get("sectionListRenderer")?
        .get("contents")?
        .get(0)?
        .get("itemSectionRenderer")?
        .get("contents")?
        .as_array()
}

fn parse_video_renderer(v: &serde_json::Value) -> Option<Video> {
    let id = v.get("videoId")?.as_str()?.to_string();
    let title = v
        .get("title")
        .and_then(|t| t.get("runs"))
        .and_then(|r| r.get(0))
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .map(decode_html_entities)
        .unwrap_or_default();

    let author = v
        .get("longBylineText")
        .and_then(|t| t.get("runs"))
        .and_then(|r| r.get(0))
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let duration = v
        .get("lengthText")
        .and_then(|t| t.get("simpleText"))
        .and_then(|t| t.as_str())
        .unwrap_or("LIVE")
        .to_string();

    let views = v
        .get("viewCountText")
        .and_then(|t| t.get("simpleText"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let published = v
        .get("publishedTimeText")
        .and_then(|t| t.get("simpleText"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let thumbnail = v
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|t| t.last())
        .and_then(|t| t.get("url"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Some(Video {
        id,
        title,
        author,
        duration,
        views,
        published,
        thumbnail,
    })
}

/// Parse video results from ytInitialData
fn parse_search_results(data: &serde_json::Value, limit: usize) -> Vec<Video> {
    let Some(items) = search_item_section_contents(data) else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let v = item.get("videoRenderer")?;
            parse_video_renderer(v)
        })
        .take(limit)
        .collect()
}

/// Search YouTube for videos (with caching)
pub async fn search_videos(query: &str, limit: usize) -> Result<Vec<Video>> {
    use crate::storage::cache::{cache_key_video_search, get_cached, set_cache};

    let cache_key = cache_key_video_search(query, limit);

    // Check cache first
    if let Some(cached) = get_cached::<Vec<Video>>(&cache_key).await {
        return Ok(cached);
    }

    // Fetch from YouTube
    let url = build_search_url(query, "video");
    let html = fetch_youtube_html(&url).await?;
    let data = extract_yt_initial_data(&html)?;
    let results = parse_search_results(&data, limit);

    if results.is_empty() {
        return Err(YtChillError::NoResults);
    }

    // Cache results (ignore errors, caching is best-effort)
    let _ = set_cache(&cache_key, &results).await;

    Ok(results)
}

/// Channel info for subscriptions
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub handle: String,
}

fn parse_channel_renderer(c: &serde_json::Value) -> Option<ChannelInfo> {
    let name = c
        .get("title")
        .and_then(|t| t.get("simpleText"))
        .and_then(|t| t.as_str())
        .map(decode_html_entities)
        .unwrap_or_default();

    // Prefer canonicalBaseUrl (e.g. "/@HandleName" or "/channel/UCxxxx").
    // Fall back to a channel/{id} form if missing.
    let handle = c
        .get("canonicalBaseUrl")
        .and_then(|u| u.as_str())
        .map(|u| u.trim_start_matches('/').to_string())
        .or_else(|| {
            c.get("channelId")
                .and_then(|id| id.as_str())
                .map(|id| format!("channel/{}", id))
        })
        .unwrap_or_default();

    if name.is_empty() || handle.is_empty() {
        return None;
    }

    Some(ChannelInfo { name, handle })
}

/// Parse channel results from ytInitialData
fn parse_channel_results(data: &serde_json::Value, limit: usize) -> Vec<ChannelInfo> {
    let Some(items) = search_item_section_contents(data) else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let c = item.get("channelRenderer")?;
            parse_channel_renderer(c)
        })
        .take(limit)
        .collect()
}

/// Search for channels
pub async fn search_channels(query: &str, limit: usize) -> Result<Vec<ChannelInfo>> {
    let url = build_search_url(query, "channel");
    let html = fetch_youtube_html(&url).await?;
    let data = extract_yt_initial_data(&html)?;
    let results = parse_channel_results(&data, limit);

    if results.is_empty() {
        return Err(YtChillError::NoResults);
    }

    Ok(results)
}

/// Build the URL for a channel's "Videos" tab.
///
/// Accepts handles in the shapes produced by `parse_channel_results`:
/// - `@HandleName`          -> `https://www.youtube.com/@HandleName/videos`
/// - `channel/UCxxxx`       -> `https://www.youtube.com/channel/UCxxxx/videos`
///
/// Also rewrites the legacy broken shape `@UCxxxx` (from before the
/// canonicalBaseUrl fix) into the `channel/UCxxxx` form so old
/// subscriptions keep working.
fn build_channel_videos_url(handle: &str) -> String {
    let path = if let Some(rest) = handle.strip_prefix("@UC") {
        format!("channel/UC{}", rest)
    } else {
        handle.to_string()
    };
    format!("https://www.youtube.com/{}/videos", path)
}

/// Parse videos from a channel "/videos" tab.
///
/// Walks `twoColumnBrowseResultsRenderer -> tabs[] -> tabRenderer.content
/// -> richGridRenderer.contents[] -> richItemRenderer.content.videoRenderer`.
/// Author is taken from `metadata.channelMetadataRenderer.title` when
/// available, otherwise `fallback_author`.
fn parse_channel_videos_tab(
    data: &serde_json::Value,
    limit: usize,
    fallback_author: &str,
) -> Vec<Video> {
    let channel_name = data
        .get("metadata")
        .and_then(|m| m.get("channelMetadataRenderer"))
        .and_then(|r| r.get("title"))
        .and_then(|t| t.as_str())
        .map(decode_html_entities)
        .unwrap_or_else(|| fallback_author.to_string());

    let tabs = data
        .get("contents")
        .and_then(|c| c.get("twoColumnBrowseResultsRenderer"))
        .and_then(|r| r.get("tabs"))
        .and_then(|t| t.as_array());

    let Some(tabs) = tabs else {
        return Vec::new();
    };

    // Prefer the tab whose richGridRenderer actually has contents; this is
    // robust to YouTube reordering tabs or localizing the "Videos" label.
    let grid_contents = tabs
        .iter()
        .filter_map(|tab| {
            tab.get("tabRenderer")
                .and_then(|t| t.get("content"))
                .and_then(|c| c.get("richGridRenderer"))
                .and_then(|g| g.get("contents"))
                .and_then(|c| c.as_array())
        })
        .find(|items| !items.is_empty());

    let Some(items) = grid_contents else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let v = item
                .get("richItemRenderer")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("videoRenderer"))?;

            let id = v.get("videoId")?.as_str()?.to_string();

            let title = v
                .get("title")
                .and_then(|t| {
                    t.get("runs")
                        .and_then(|r| r.get(0))
                        .and_then(|r| r.get("text"))
                        .and_then(|t| t.as_str())
                        .or_else(|| t.get("simpleText").and_then(|t| t.as_str()))
                })
                .map(decode_html_entities)
                .unwrap_or_default();

            let duration = v
                .get("lengthText")
                .and_then(|t| t.get("simpleText"))
                .and_then(|t| t.as_str())
                .unwrap_or("LIVE")
                .to_string();

            let views = v
                .get("viewCountText")
                .and_then(|t| t.get("simpleText"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let published = v
                .get("publishedTimeText")
                .and_then(|t| t.get("simpleText"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let thumbnail = v
                .get("thumbnail")
                .and_then(|t| t.get("thumbnails"))
                .and_then(|t| t.as_array())
                .and_then(|t| t.last())
                .and_then(|t| t.get("url"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            Some(Video {
                id,
                title,
                author: channel_name.clone(),
                duration,
                views,
                published,
                thumbnail,
            })
        })
        .take(limit)
        .collect()
}

/// Fetch recent videos from a channel's "Videos" tab.
pub async fn fetch_channel_videos(channel_handle: &str, limit: usize) -> Result<Vec<Video>> {
    use crate::storage::cache::{cache_key_channel_videos, get_cached, set_cache};

    let cache_key = cache_key_channel_videos(channel_handle, limit);

    if let Some(cached) = get_cached::<Vec<Video>>(&cache_key).await {
        return Ok(cached);
    }

    let url = build_channel_videos_url(channel_handle);
    let html = fetch_youtube_html(&url).await?;
    let data = extract_yt_initial_data(&html)?;
    let results = parse_channel_videos_tab(&data, limit, channel_handle);

    if results.is_empty() {
        return Err(YtChillError::YouTubeParse(format!(
            "no videos found for channel {}",
            channel_handle
        )));
    }

    let _ = set_cache(&cache_key, &results).await;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tracing_test::traced_test;

    #[test]
    fn test_build_search_url() {
        let url = build_search_url("lofi beats", "video");
        assert!(url.contains("search_query=lofi%20beats"));
        assert!(url.contains("sp=EgIQAQ"));
    }

    #[test]
    fn build_search_url_channel_filter_includes_channel_sp() {
        let url = build_search_url("jazz radio", "channel");
        assert!(url.contains("search_query=jazz%20radio"));
        assert!(url.contains("sp=EgIQAg"));
    }

    #[test]
    fn build_search_url_unknown_filter_uses_empty_sp() {
        let url = build_search_url("anything", "other");
        assert!(url.contains("search_query=anything"));
        assert!(url.ends_with("sp="));
    }

    #[test]
    fn build_channel_videos_url_handles_all_shapes() {
        assert_eq!(
            build_channel_videos_url("@Handle"),
            "https://www.youtube.com/@Handle/videos"
        );
        assert_eq!(
            build_channel_videos_url("channel/UC_abc"),
            "https://www.youtube.com/channel/UC_abc/videos"
        );
        // Legacy broken shape from pre-item-9 subscriptions must be rewritten.
        assert_eq!(
            build_channel_videos_url("@UC_JhYV43bqoR_P6z2aB80hA"),
            "https://www.youtube.com/channel/UC_JhYV43bqoR_P6z2aB80hA/videos"
        );
    }

    fn make_search_fixture() -> serde_json::Value {
        json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "itemSectionRenderer": {
                                    "contents": [{
                                        "videoRenderer": {
                                            "videoId": "abc123",
                                            "title": {"runs": [{"text": "Hello &amp; goodbye"}]},
                                            "longBylineText": {"runs": [{"text": "Test Author"}]},
                                            "lengthText": {"simpleText": "10:15"},
                                            "viewCountText": {"simpleText": "1,234 views"},
                                            "publishedTimeText": {"simpleText": "2 days ago"},
                                            "thumbnail": {"thumbnails": [
                                                {"url": "http://example.com/small.jpg"},
                                                {"url": "http://example.com/large.jpg"}
                                            ]}
                                        }
                                    }]
                                }
                            }]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn parse_search_results_extracts_fields_and_decodes_entities() {
        let data = make_search_fixture();
        let videos = parse_search_results(&data, 10);
        assert_eq!(videos.len(), 1);
        let v = &videos[0];
        assert_eq!(v.id, "abc123");
        assert_eq!(v.title, "Hello & goodbye");
        assert_eq!(v.author, "Test Author");
        assert_eq!(v.duration, "10:15");
        assert_eq!(v.views, "1,234 views");
        assert_eq!(v.published, "2 days ago");
        assert_eq!(v.thumbnail, "http://example.com/large.jpg");
    }

    #[test]
    fn parse_search_results_respects_limit() {
        let mut data = make_search_fixture();
        // Duplicate the renderer three times so limit=2 actually trims.
        let contents = data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]
            ["sectionListRenderer"]["contents"][0]["itemSectionRenderer"]["contents"]
            .as_array()
            .unwrap()
            .clone();
        let first = contents[0].clone();
        data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]
            ["contents"][0]["itemSectionRenderer"]["contents"] =
            json!([first.clone(), first.clone(), first]);
        assert_eq!(parse_search_results(&data, 2).len(), 2);
    }

    #[test]
    fn parse_search_results_missing_fields_returns_empty() {
        assert!(parse_search_results(&json!({}), 10).is_empty());
    }

    #[test]
    fn parse_search_results_defaults_duration_to_live() {
        let mut data = make_search_fixture();
        data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]
            ["sectionListRenderer"]["contents"][0]["itemSectionRenderer"]["contents"][0]
            ["videoRenderer"]
            .as_object_mut()
            .unwrap()
            .remove("lengthText");
        let videos = parse_search_results(&data, 10);
        assert_eq!(videos[0].duration, "LIVE");
    }

    fn channel_search_fixture(canonical: Option<&str>, channel_id: &str) -> serde_json::Value {
        let mut renderer = json!({
            "title": {"simpleText": "Some Channel"},
            "channelId": channel_id,
            "subscriberCountText": {"simpleText": "1M subscribers"}
        });
        if let Some(c) = canonical {
            renderer["canonicalBaseUrl"] = json!(c);
        }
        json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "itemSectionRenderer": {
                                    "contents": [{ "channelRenderer": renderer }]
                                }
                            }]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn parse_channel_results_prefers_canonical_base_url_handle() {
        let data = channel_search_fixture(Some("/@TheHandle"), "UC_ignored");
        let channels = parse_channel_results(&data, 10);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].handle, "@TheHandle");
        assert_eq!(channels[0].name, "Some Channel");
    }

    #[test]
    fn parse_channel_results_accepts_canonical_channel_url() {
        let data = channel_search_fixture(Some("/channel/UC_abc"), "UC_abc");
        let channels = parse_channel_results(&data, 10);
        assert_eq!(channels[0].handle, "channel/UC_abc");
    }

    #[test]
    fn parse_channel_results_falls_back_to_channel_id_form() {
        let data = channel_search_fixture(None, "UC_fallback");
        let channels = parse_channel_results(&data, 10);
        assert_eq!(channels[0].handle, "channel/UC_fallback");
    }

    fn channel_videos_fixture() -> serde_json::Value {
        json!({
            "metadata": {
                "channelMetadataRenderer": {"title": "Channel Title"}
            },
            "contents": {
                "twoColumnBrowseResultsRenderer": {
                    "tabs": [
                        // Empty tab should be skipped.
                        {"tabRenderer": {"content": {"richGridRenderer": {"contents": []}}}},
                        {"tabRenderer": {"content": {"richGridRenderer": {"contents": [
                            {"richItemRenderer": {"content": {"videoRenderer": {
                                "videoId": "vid1",
                                "title": {"runs": [{"text": "First"}]},
                                "lengthText": {"simpleText": "5:00"},
                                "viewCountText": {"simpleText": "10 views"},
                                "publishedTimeText": {"simpleText": "1 hour ago"},
                                "thumbnail": {"thumbnails": [
                                    {"url": "http://example.com/t1.jpg"}
                                ]}
                            }}}},
                            {"richItemRenderer": {"content": {"videoRenderer": {
                                "videoId": "vid2",
                                "title": {"simpleText": "Second via simpleText"},
                                // No lengthText -> LIVE
                                "thumbnail": {"thumbnails": [
                                    {"url": "http://example.com/t2.jpg"}
                                ]}
                            }}}},
                            // Non-video entry (e.g. reelItemRenderer) should be filtered.
                            {"richItemRenderer": {"content": {"reelItemRenderer": {
                                "videoId": "reel1"
                            }}}}
                        ]}}}}
                    ]
                }
            }
        })
    }

    #[test]
    fn parse_channel_videos_tab_skips_empty_tabs_and_extracts() {
        let data = channel_videos_fixture();
        let videos = parse_channel_videos_tab(&data, 10, "@fallback");
        assert_eq!(videos.len(), 2);
        assert_eq!(videos[0].id, "vid1");
        assert_eq!(videos[0].title, "First");
        assert_eq!(videos[0].author, "Channel Title");
        assert_eq!(videos[0].duration, "5:00");
        assert_eq!(videos[1].id, "vid2");
        assert_eq!(videos[1].title, "Second via simpleText");
        assert_eq!(videos[1].duration, "LIVE");
    }

    #[test]
    fn parse_channel_videos_tab_falls_back_to_provided_author() {
        let mut data = channel_videos_fixture();
        data["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("channelMetadataRenderer");
        let videos = parse_channel_videos_tab(&data, 10, "@fallback");
        assert_eq!(videos[0].author, "@fallback");
    }

    #[test]
    fn parse_channel_videos_tab_respects_limit() {
        let data = channel_videos_fixture();
        assert_eq!(parse_channel_videos_tab(&data, 1, "@x").len(), 1);
    }

    #[test]
    fn parse_channel_videos_tab_empty_on_missing_structure() {
        assert!(parse_channel_videos_tab(&json!({}), 10, "@x").is_empty());
    }

    #[test]
    fn extract_yt_initial_data_parses_first_script_match() {
        let html = r#"<!doctype html><script>var ytInitialData = {"ok":true};</script><p>x</p>"#;
        let data = extract_yt_initial_data(html).unwrap();
        assert_eq!(data, json!({"ok": true}));
    }

    #[test]
    fn extract_yt_initial_data_ignores_noise_before_marker() {
        let html = concat!(
            "<html><script>var x=1;</script>",
            r#"<script>var ytInitialData = {"n":2};</script></html>"#
        );
        let data = extract_yt_initial_data(html).unwrap();
        assert_eq!(data["n"], json!(2));
    }

    #[traced_test]
    #[test]
    fn extract_yt_initial_data_emits_debug_on_regex_miss() {
        let _ = extract_yt_initial_data("plain text without marker");
        assert!(logs_contain("yt_initial_data_regex_miss"));
    }

    #[traced_test]
    #[test]
    fn extract_yt_initial_data_emits_debug_on_json_error() {
        let html = r#"<script>var ytInitialData = not_valid_json;</script>"#;
        let err = extract_yt_initial_data(html).unwrap_err();
        assert!(err.to_string().contains("parse"), "unexpected error: {err}");
        assert!(logs_contain("yt_initial_data_json_error"));
    }

    #[test]
    fn parse_search_results_empty_when_item_section_has_no_contents() {
        let data = json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "itemSectionRenderer": {"contents": []}
                            }]
                        }
                    }
                }
            }
        });
        assert!(parse_search_results(&data, 10).is_empty());
    }

    #[test]
    fn parse_channel_results_respects_limit() {
        let mut data = channel_search_fixture(Some("/@A"), "UCa");
        let item = data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]
            ["sectionListRenderer"]["contents"][0]["itemSectionRenderer"]["contents"][0]
            .clone();
        data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]
            ["contents"][0]["itemSectionRenderer"]["contents"] = json!([item.clone(), item]);
        assert_eq!(parse_channel_results(&data, 1).len(), 1);
    }

    #[test]
    fn parse_channel_results_decodes_html_entities_in_name() {
        let data = json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "itemSectionRenderer": {
                                    "contents": [{
                                        "channelRenderer": {
                                            "title": {"simpleText": "AT&amp;T Labs"},
                                            "canonicalBaseUrl": "/@ATT",
                                            "channelId": "UCx"
                                        }
                                    }]
                                }
                            }]
                        }
                    }
                }
            }
        });
        let ch = &parse_channel_results(&data, 5)[0];
        assert_eq!(ch.name, "AT&T Labs");
        assert_eq!(ch.handle, "@ATT");
    }

    #[test]
    fn parse_search_results_skips_video_renderer_without_id() {
        let mut data = make_search_fixture();
        let good = data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]
            ["sectionListRenderer"]["contents"][0]["itemSectionRenderer"]["contents"][0]
            .clone();
        data["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]
            ["contents"][0]["itemSectionRenderer"]["contents"] = json!([
            json!({"videoRenderer": {
                "title": {"runs": [{"text": "Missing id"}]},
                "longBylineText": {"runs": [{"text": "x"}]}
            }}),
            good
        ]);
        let videos = parse_search_results(&data, 10);
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].id, "abc123");
    }

    #[test]
    fn parse_channel_results_skips_channel_renderer_without_handle() {
        let data = json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [{
                                "itemSectionRenderer": {
                                    "contents": [{
                                        "channelRenderer": {
                                            "title": {"simpleText": "No id or url"},
                                            "subscriberCountText": {"simpleText": "1"}
                                        }
                                    }]
                                }
                            }]
                        }
                    }
                }
            }
        });
        assert!(parse_channel_results(&data, 10).is_empty());
    }
}
