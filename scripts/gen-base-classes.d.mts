/**
 * The generator is a plain script so it can run without the TypeScript pipeline; this is what it
 * offers to a test that wants to check the files it writes are current.
 */

/** What each generated file should hold right now, by absolute path. */
export function generate(): Record<string, string>;
