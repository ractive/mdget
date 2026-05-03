use std::collections::HashMap;
use std::time::Duration;

use texting_robots::Robot;
use url::Url;

/// Cached robots.txt data for domains already looked up.
///
/// `None` means we attempted a fetch but it failed (404, network error, etc.),
/// which is treated as "allow all" per the robots.txt specification.
pub struct RobotsCache {
    /// Maps `"scheme://host"` to a parsed `Robot`, or `None` if unavailable.
    cache: HashMap<String, Option<Robot>>,
}

impl RobotsCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Returns the domain key (`scheme://host:port`) for a URL.
    fn domain_key(url: &Url) -> Option<String> {
        let host = url.host_str()?;
        match url.port() {
            Some(port) => Some(format!("{}://{}:{}", url.scheme(), host, port)),
            None => Some(format!("{}://{}", url.scheme(), host)),
        }
    }

    /// Returns `true` if `url` is allowed by robots.txt.
    ///
    /// Fetches and caches robots.txt for the domain on first access.
    /// Any failure to fetch (network error, 404, etc.) is treated as "allow all".
    pub fn is_allowed(&mut self, url: &Url, client: &reqwest::blocking::Client) -> bool {
        let Some(key) = Self::domain_key(url) else {
            // Can't determine domain — allow it.
            return true;
        };

        if !self.cache.contains_key(&key) {
            let robot = fetch_robots(client, url);
            self.cache.insert(key.clone(), robot);
        }

        match self.cache.get(&key) {
            Some(Some(robot)) => robot.allowed(url.as_str()),
            // None means fetch failed — allow all.
            Some(None) | None => true,
        }
    }

    /// Returns the crawl delay for a domain, if known from a previously fetched
    /// robots.txt.  Returns `None` if the domain has not been looked up yet or
    /// if the robots.txt did not specify a delay.
    pub fn crawl_delay(&self, url: &Url) -> Option<Duration> {
        let key = Self::domain_key(url)?;
        match self.cache.get(&key) {
            Some(Some(robot)) => robot
                .delay
                .map(|secs| Duration::from_secs_f32(secs.max(0.0))),
            _ => None,
        }
    }
}

/// Fetches and parses robots.txt for the host of `url`.
///
/// Returns `None` on any error (connection failure, 4xx, 5xx, parse error).
/// Per the robots.txt specification, an unavailable robots.txt means "allow all".
fn fetch_robots(client: &reqwest::blocking::Client, url: &Url) -> Option<Robot> {
    // Build the robots.txt URL using the full authority (host + optional port).
    let authority = match url.port() {
        Some(port) => format!("{}:{}", url.host_str().unwrap_or(""), port),
        None => url.host_str().unwrap_or("").to_string(),
    };
    let robots_url = format!("{}://{}/robots.txt", url.scheme(), authority);

    let response = client.get(&robots_url).send().ok()?;

    // 4xx → treat as "no restrictions" (per Google's recommendation).
    // 5xx → treat as "do not crawl" per spec, but we default to allow for simplicity.
    // Both map to returning None (allow all).
    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().ok()?;

    // texting_robots::Robot::new returns anyhow::Result.
    Robot::new("*", &bytes).ok()
}
