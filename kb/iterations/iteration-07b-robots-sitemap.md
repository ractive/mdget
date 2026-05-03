---
title: robots.txt and sitemap support
type: iteration
date: 2026-05-03
tags:
  - iteration
  - crawling
  - robots
  - sitemap
status: in-progress
branch: iter-7b/robots-sitemap
---

## Goal

Layer robots.txt compliance and sitemap.xml discovery onto the crawl engine from [[iteration-07a-crawl-engine]]. After this, `mdget crawl` is a responsible, well-behaved crawler.

## CLI Interface (additions)

```
mdget crawl https://docs.example.com               # now respects robots.txt by default
mdget crawl --ignore-robots https://example.com     # override robots.txt
mdget crawl --sitemap https://docs.example.com      # discover pages via sitemap.xml
```

## Defaults

| Setting | Default | Flag |
|---------|---------|------|
| Respect robots.txt | yes | `--ignore-robots` to override |
| Crawl-delay from robots.txt | yes (overrides `--delay` if higher) | — |
| Sitemap discovery | off | `--sitemap` to enable |

## Crate Decisions

### robots.txt: `texting_robots` (v0.2.2)

Chosen over `robotstxt` because:
- **Parse once, query many**: `Robot::new(ua, body)` then `robot.allowed(url)` — fits crawler lifecycle
- **Crawl-delay support**: exposes `delay` field, which `robotstxt` does not
- **Extensively tested**: validated against 34M+ real robots.txt files, includes Google C++ and Moz reppy test suites
- **`robotstxt` has a known panic** on char boundary slicing (issue #5, open since 2021) — dealbreaker

Trade-off: heavier deps (`nom`, `regex`, `bstr`) but `regex` is already transitive in the workspace.

### Sitemap XML: `quick-xml` (v0.37)

- Pure Rust, single mandatory dep (`memchr`)
- Serde support via `serialize` feature — deserialize sitemap XML directly into structs
- Actively maintained (Feb 2026)
- Streaming reader available but serde is cleaner for small-to-medium sitemaps

## Tasks

- [x] Add `texting_robots` dependency to `mdget-core`
- [x] Fetch and cache robots.txt per domain during crawl (one fetch per domain, reuse for all URLs)
- [x] Filter URLs through `Robot::allowed()` before fetching
- [x] Honour `Crawl-delay` directive (use the higher of robots.txt delay and `--delay` flag)
- [x] Add `--ignore-robots` flag to bypass robots.txt
- [x] Add `quick-xml` dependency with `serialize` feature to `mdget-core`
- [x] Implement sitemap.xml parser: support `<urlset>` and `<sitemapindex>` (nested sitemaps)
- [x] Add `--sitemap` flag: fetch sitemap.xml, add discovered URLs to crawl queue
- [x] When `--sitemap` is used with `--depth 0`, fetch sitemap URLs + start page (no link following)
- [x] Add e2e tests: robots.txt blocking, crawl-delay, sitemap discovery
- [x] Run quality gates

## Design Decisions

- **robots.txt cached per domain**: fetched once at crawl start (or when a new domain is encountered with `--follow-external`), stored in a `HashMap<String, Robot>`. Avoids re-fetching on every URL check.
- **Crawl-delay is a floor**: if robots.txt says `Crawl-delay: 5` and user passes `--delay 1`, use 5. If user passes `--delay 10`, use 10. The higher value wins.
- **Sitemap + crawl are complementary**: `--sitemap` seeds the BFS queue with sitemap URLs. Link following still happens unless `--depth 0`. This lets users do sitemap-only fetches or sitemap-seeded crawls.
- **Nested sitemaps**: sitemap index files (`<sitemapindex>`) reference child sitemaps. Follow one level of nesting (fetch child sitemaps, but don't recurse further — real-world sitemap indexes rarely nest deeper).
