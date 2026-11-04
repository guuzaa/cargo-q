/**
 * The worker behind the page.
 *
 * The page itself is static: this only answers the one question that
 * cannot be answered at build time — what is the latest version on
 * crates.io — and hands everything else to the assets.
 */

/// <reference types="@cloudflare/workers-types" />

export interface Env {
  ASSETS: Fetcher;
}

const CRATE = 'https://crates.io/api/v1/crates/cargo-q';

/** How long a version is kept before it is looked up again. */
const FRESH_FOR = 3600;

interface Crate {
  crate: { max_version: string };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // Cloudflare adds a default cache to the Cache API.
    const cache = (caches as CacheStorage & { default: Cache }).default;

    if (url.pathname === '/api/version') {
      const cached = await cache.match(request);
      if (cached) return cached;

      const response = await fetch(CRATE, {
        headers: { 'user-agent': 'cargo-q.paulden.site (the landing page of cargo-q)' },
      });
      if (!response.ok) {
        return new Response('crates.io is not answering', { status: 502 });
      }

      const crate = (await response.json()) as Crate;
      const body = JSON.stringify({ version: crate.crate.max_version });

      const answer = new Response(body, {
        headers: {
          'content-type': 'application/json; charset=utf-8',
          'cache-control': `public, max-age=${FRESH_FOR}`,
        },
      });
      await cache.put(request, answer.clone());
      return answer;
    }

    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
