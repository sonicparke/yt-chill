//! YouTube scraping and parsing

use crate::error::{Result, YtChillError};
use crate::types::Video;
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
    let re = regex::Regex::new(r"var ytInitialData = (.+?);</script>").expect("Invalid regex");

    let captures = re
        .captures(html)
        .ok_or_else(|| YtChillError::YouTubeParse("Failed to find ytInitialData".into()))?;

    let json_str = captures.get(1).unwrap().as_str();
    serde_json::from_str(json_str)
        .map_err(|e| YtChillError::YouTubeParse(format!("Failed to parse ytInitialData: {}", e)))
}

/// Decode HTML entities in a string
fn decode_html_entities(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}

/// Parse video results from ytInitialData
fn parse_search_results(data: &serde_json::Value, limit: usize) -> Vec<Video> {
    let items = data
        .get("contents")
        .and_then(|c| c.get("twoColumnSearchResultsRenderer"))
        .and_then(|r| r.get("primaryContents"))
        .and_then(|p| p.get("sectionListRenderer"))
        .and_then(|s| s.get("contents"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("itemSectionRenderer"))
        .and_then(|i| i.get("contents"))
        .and_then(|c| c.as_array());

    let Some(items) = items else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let v = item.get("videoRenderer")?;

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
        })
        .take(limit)
        .collect()
}

/// Search YouTube for videos (with caching)
pub async fn search_videos(query: &str, limit: usize) -> Result<Vec<Video>> {
    use crate::storage::cache::{get_cache_key, get_cached, set_cache};

    // Generate cache key from query + limit
    let cache_key = get_cache_key(&format!("video:{}:{}", query, limit));

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

/// Parse channel results from ytInitialData
fn parse_channel_results(data: &serde_json::Value, limit: usize) -> Vec<ChannelInfo> {
    let items = data
        .get("contents")
        .and_then(|c| c.get("twoColumnSearchResultsRenderer"))
        .and_then(|r| r.get("primaryContents"))
        .and_then(|p| p.get("sectionListRenderer"))
        .and_then(|s| s.get("contents"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("itemSectionRenderer"))
        .and_then(|i| i.get("contents"))
        .and_then(|c| c.as_array());

    let Some(items) = items else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let c = item.get("channelRenderer")?;

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
    use crate::storage::cache::{get_cache_key, get_cached, set_cache};

    let cache_key = get_cache_key(&format!("channel:{}:{}", channel_handle, limit));

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

    #[test]
    fn test_build_search_url() {
        let url = build_search_url("lofi beats", "video");
        assert!(url.contains("search_query=lofi%20beats"));
        assert!(url.contains("sp=EgIQAQ"));
    }
}
