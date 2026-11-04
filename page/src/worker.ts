/**
 * The worker behind the page.
 *
 * The page itself is static: the worker only hands the requests to the
 * static assets.
 */

/// <reference types="@cloudflare/workers-types" />

export interface Env {
  ASSETS: Fetcher;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
