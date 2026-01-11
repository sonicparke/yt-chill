//! YouTube scraping and parsing

use crate::error::{Result, YtChillError};
use crate::types::Video;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

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
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
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
    let re = regex::Regex::new(r"var ytInitialData = (.+?);</script>")
        .expect("Invalid regex");

    let captures = re.captures(html).ok_or_else(|| {
        YtChillError::YouTubeParse("Failed to find ytInitialData".into())
    })?;

    let json_str = captures.get(1).unwrap().as_str();
    serde_json::from_str(json_str).map_err(|e| {
        YtChillError::YouTubeParse(format!("Failed to parse ytInitialData: {}", e))
    })
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

            // Try to get handle, fall back to channel ID
            let handle = c
                .get("subscriberCountText")
                .and_then(|_| c.get("channelId"))
                .and_then(|id| id.as_str())
                .map(|id| format!("@{}", id))
                .or_else(|| {
                    c.get("channelId")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
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

/// Fetch recent videos from a channel
pub async fn fetch_channel_videos(channel_handle: &str, limit: usize) -> Result<Vec<Video>> {
    use crate::storage::cache::{get_cache_key, get_cached, set_cache};

    // Generate cache key
    let cache_key = get_cache_key(&format!("channel:{}:{}", channel_handle, limit));

    // Check cache first
    if let Some(cached) = get_cached::<Vec<Video>>(&cache_key).await {
        return Ok(cached);
    }

    // Build channel URL - search for channel videos
    let search_query = format!("{} ", channel_handle);
    let url = build_search_url(&search_query, "video");
    let html = fetch_youtube_html(&url).await?;
    let data = extract_yt_initial_data(&html)?;
    let results = parse_search_results(&data, limit);

    // Cache results
    if !results.is_empty() {
        let _ = set_cache(&cache_key, &results).await;
    }

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

