// ---------------------------------------------------------------------------
// myinstants.com integration
// ---------------------------------------------------------------------------
// Search and preview of sounds hosted on https://www.myinstants.com. The site
// does not expose a stable JSON API anymore, so we scrape the public search
// pages (HTML) over HTTPS from the main process — fetching from the renderer
// would be blocked by CORS, and scraping here keeps the dirty parsing out of
// the UI. Sound files themselves are served directly from
// `https://www.myinstants.com/media/sounds/...` and can be played back in the
// renderer with a plain `HTMLAudioElement`.

/** Base origin for all myinstants URLs (pages + media). */
export const MYINSTANTS_ORIGIN = "https://www.myinstants.com";

/**
 * Browser-like User-Agent. Some CDNs / HTML routes return a challenge page for
 * bare `node-fetch` UAs, so we present as a desktop browser.
 */
export const MYINSTANTS_USER_AGENT =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36";

/** Number of `<div class="instant">` entries myinstants renders per page. */
export const RESULTS_PER_PAGE = 36;

/** A single sound entry parsed from a search page. */
export interface RemoteSound {
  /** Numeric myinstants instant id (the `favorite(...)` argument) if found. */
  id: string;
  /** Title rendered in the page link, decoded + trimmed. */
  title: string;
  /** Page slug (/en/instant/<slug>/). Falls back to the media basename. */
  slug: string;
  /** `/media/sounds/<filename>` path on myinstants. */
  path: string;
  /** Bare media filename, e.g. `dry-fart.mp3`. */
  file: string;
  /** Absolute URL the renderer can hand to `new Audio(url)` for previewing. */
  url: string;
}

/** One page of search results plus a "more pages?" hint. */
export interface SearchPage {
  results: RemoteSound[];
  /** The page number that was requested. */
  page: number;
  /** Best-effort "has more" flag: the site has no pagination metadata, so a
   * full-size page (RESULTS_PER_PAGE items) is treated as "maybe more". */
  hasMore: boolean;
}

/** Decode the small handful of HTML entities myinstants uses in titles. */
function decodeEntities(text: string): string {
  return text
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_match, code) => String.fromCharCode(Number(code)))
    .replace(/&#x([0-9a-fA-F]+);/g, (_match, code) => String.fromCharCode(parseInt(code, 16)));
}

// Search page layout (per `<div class="instant">` card):
//   <button class="small-button" onclick="play('/media/sounds/foo.mp3', 'loader-123', 'foo-slug')" ...>
//   <a href="/en/instant/foo-slug/" class="instant-link link-secondary">Foo Title</a>
//   <button ... onclick="favorite('123')" ... />
// We grab each regex globally and pair-by-index — the three are always in sync
// within a card; pair-by-index breaks only if myinstants reorders them.
const PLAY_RE =
  /<button class="small-button" onclick="play\('([^']+)',\s*'loader-[^']*',\s*'([^']*)'\)"/g;
// link text is plain text, but be defensive: trim surrounding whitespace and
// collapse nested tags by stripping inner `<...>` inside the captured span.
const TITLE_RE = /<a[^>]*class="instant-link[^"]*"[^>]*>([\s\S]*?)<\/a>/g;
const FAVORITE_RE = /favorite\('(\d+)'\)/g;

/** Parse the instants list out of a search (or category) page's HTML. */
export function parseInstants(html: string): RemoteSound[] {
  const plays = [...html.matchAll(PLAY_RE)];
  const titles = [...html.matchAll(TITLE_RE)];
  const favorites = [...html.matchAll(FAVORITE_RE)];

  const count = Math.min(plays.length, titles.length);
  const results: RemoteSound[] = [];
  for (let index = 0; index < count; index += 1) {
    const path = plays[index]![1];
    const slug = plays[index]![2] || "";
    const rawTitle = titles[index]![1];

    const title = decodeEntities(rawTitle)
      .replace(/<[^>]+>/g, "")
      .replace(/\s+/g, " ")
      .trim();
    const id = favorites[index]?.[1] ?? slug;

    const file = path.split("/").pop() ?? slug;
    const url = `${MYINSTANTS_ORIGIN}${path}`;

    // Skip the occasional number-less or path-less match from a stray card.
    if (!path || path.indexOf("/media/sounds/") !== 0) continue;

    results.push({ id, title, slug, path, file, url });
  }
  return results;
}

/**
 * Fetch a page of search results from myinstants. Throws on network/HTTP
 * errors so the renderer can surface them; never rejects on "no results"
 * (returns an empty page instead — an ended result set looks the same as a
 * search with no hits).
 */
export async function searchMyinstants(query: string, page = 1): Promise<SearchPage> {
  const safePage = Math.max(1, Math.floor(page) || 1);
  const url = new URL(`${MYINSTANTS_ORIGIN}/en/search/`);
  url.searchParams.set("name", query);
  if (safePage > 1) url.searchParams.set("page", String(safePage));

  let response: Response;
  try {
    response = await fetch(url, {
      headers: {
        "User-Agent": MYINSTANTS_USER_AGENT,
        Accept: "text/html,application/xhtml+xml",
        "Accept-Language": "en-US,en;q=0.8",
      },
      redirect: "follow",
    });
  } catch (error) {
    throw new Error(
      `Could not reach myinstants.com: ${error instanceof Error ? error.message : String(error)}`,
      { cause: error },
    );
  }
  if (!response.ok) {
    // myinstants returns 404 for mismatched queries (with an empty result
    // page), so surface that as a normal empty search rather than an error.
    if (response.status === 404) {
      return { results: [], page: safePage, hasMore: false };
    }
    throw new Error(`myinstants.com returned status ${response.status}`);
  }

  const html = await response.text();
  const results = parseInstants(html);
  return { results, page: safePage, hasMore: results.length === RESULTS_PER_PAGE };
}
