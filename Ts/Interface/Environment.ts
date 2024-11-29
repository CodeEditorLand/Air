import type { KVNamespace } from "@cloudflare/workers-types/experimental/index.js";

/**
 * @module Worker
 *
 */
export default interface Interface {
	HTML: KVNamespace;

	CSS: KVNamespace;

	TypeScript: KVNamespace;
}
