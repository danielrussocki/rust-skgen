//! Crawl policy evaluation boundary.

use crate::fetch::{DocumentFetcher, FetchError};
use url::Url;

/// Marks a component that evaluates whether a URL may be crawled.
pub trait CrawlPolicy {
    /// Returns whether the configured crawler may request the URL.
    fn allows(&self, url: &Url) -> Result<bool, RobotsError>;
}

/// Evaluates access conditions applicable to a documentation URL.
pub trait AccessPolicy {
    /// Returns whether the URL is permitted by the applicable access conditions.
    fn allows(&self, url: &Url) -> bool;
}

/// Requires both robots.txt and applicable access conditions to permit a URL.
pub struct CombinedCrawlPolicy<R, A> {
    robots_policy: R,
    access_policy: A,
}

impl<R, A> CombinedCrawlPolicy<R, A> {
    /// Combines a robots.txt policy with applicable access conditions.
    pub fn new(robots_policy: R, access_policy: A) -> Self {
        Self {
            robots_policy,
            access_policy,
        }
    }
}

impl<R: CrawlPolicy, A: AccessPolicy> CrawlPolicy for CombinedCrawlPolicy<R, A> {
    fn allows(&self, url: &Url) -> Result<bool, RobotsError> {
        Ok(self.robots_policy.allows(url)? && self.access_policy.allows(url))
    }
}

/// Error returned while retrieving or evaluating a robots policy.
#[derive(Debug)]
pub enum RobotsError {
    /// The robots document URL could not be derived from a page URL.
    InvalidRobotsUrl(url::ParseError),
    /// The robots document could not be retrieved.
    Fetch(FetchError),
    /// The robots document did not return a successful HTTP status.
    UnexpectedStatus(u16),
}

impl std::fmt::Display for RobotsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRobotsUrl(error) => write!(formatter, "invalid robots.txt URL: {error}"),
            Self::Fetch(error) => write!(formatter, "failed to retrieve robots.txt: {error}"),
            Self::UnexpectedStatus(status) => {
                write!(
                    formatter,
                    "robots.txt returned unexpected HTTP status {status}"
                )
            }
        }
    }
}

impl std::error::Error for RobotsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidRobotsUrl(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::UnexpectedStatus(_) => None,
        }
    }
}

/// A robots.txt policy backed by an HTTP document fetcher.
pub struct RobotsTxtPolicy<F> {
    fetcher: F,
    user_agent: String,
}

impl<F> RobotsTxtPolicy<F> {
    /// Creates a policy evaluator for the supplied crawler user agent.
    pub fn new(fetcher: F, user_agent: impl Into<String>) -> Self {
        Self {
            fetcher,
            user_agent: user_agent.into(),
        }
    }
}

impl<F: DocumentFetcher> CrawlPolicy for RobotsTxtPolicy<F> {
    fn allows(&self, url: &Url) -> Result<bool, RobotsError> {
        let robots_url = url
            .join("/robots.txt")
            .map_err(RobotsError::InvalidRobotsUrl)?;
        let document = self.fetcher.fetch(robots_url).map_err(RobotsError::Fetch)?;
        if !(200..300).contains(&document.status()) {
            return Err(RobotsError::UnexpectedStatus(document.status()));
        }

        Ok(RobotsRules::parse(document.body()).allows(url, &self.user_agent))
    }
}

#[derive(Default)]
struct RobotsRules {
    groups: Vec<RobotsGroup>,
}

impl RobotsRules {
    fn parse(document: &str) -> Self {
        let mut groups = Vec::new();
        let mut agents = Vec::new();
        let mut rules = Vec::new();
        let mut has_rules = false;

        for line in document.lines() {
            let line = line.split_once('#').map_or(line, |(value, _)| value).trim();
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            let field = field.trim();
            let value = value.trim();

            if field.eq_ignore_ascii_case("user-agent") {
                if has_rules {
                    groups.push(RobotsGroup { agents, rules });
                    agents = Vec::new();
                    rules = Vec::new();
                    has_rules = false;
                }
                agents.push(value.to_ascii_lowercase());
            } else if !agents.is_empty()
                && (field.eq_ignore_ascii_case("allow") || field.eq_ignore_ascii_case("disallow"))
                && !value.is_empty()
            {
                rules.push(RobotsRule {
                    pattern: value.to_owned(),
                    allowed: field.eq_ignore_ascii_case("allow"),
                });
                has_rules = true;
            }
        }

        if !agents.is_empty() {
            groups.push(RobotsGroup { agents, rules });
        }

        Self { groups }
    }

    fn allows(&self, url: &Url, user_agent: &str) -> bool {
        let user_agent = user_agent.to_ascii_lowercase();
        let specificity = self
            .groups
            .iter()
            .flat_map(|group| group.agents.iter())
            .filter_map(|agent| user_agent_match_length(agent, &user_agent))
            .max();
        let Some(specificity) = specificity else {
            return true;
        };

        let path = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        self.groups
            .iter()
            .filter(|group| {
                group
                    .agents
                    .iter()
                    .any(|agent| user_agent_match_length(agent, &user_agent) == Some(specificity))
            })
            .flat_map(|group| &group.rules)
            .filter(|rule| matches_pattern(&rule.pattern, &path))
            .max_by_key(|rule| (rule.pattern.len(), rule.allowed))
            .is_none_or(|rule| rule.allowed)
    }
}

struct RobotsGroup {
    agents: Vec<String>,
    rules: Vec<RobotsRule>,
}

struct RobotsRule {
    pattern: String,
    allowed: bool,
}

fn user_agent_match_length(agent: &str, user_agent: &str) -> Option<usize> {
    if agent == "*" {
        Some(0)
    } else {
        user_agent.starts_with(agent).then_some(agent.len())
    }
}

fn matches_pattern(pattern: &str, path: &str) -> bool {
    let anchored = pattern.ends_with('$');
    let pattern = pattern.strip_suffix('$').unwrap_or(pattern);
    let mut remainder = path;

    for segment in pattern.split('*') {
        if segment.is_empty() {
            continue;
        }
        let Some(position) = remainder.find(segment) else {
            return false;
        };
        if remainder.len() == path.len() && position != 0 {
            return false;
        }
        remainder = &remainder[position + segment.len()..];
    }

    !anchored || remainder.is_empty()
}
