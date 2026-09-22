/**
 * React's window onto the live object model. Every `RuntimeObject` is its own external store,
 * so a control subscribes to itself and nothing else: `THISFORM.lblGreeting.Caption = x`
 * re-renders one label instead of the whole form.
 */

import { useMemo, useSyncExternalStore } from 'react';
import type { PropValue } from '@shared/form/schema';
import type { Desktop, RuntimeObject } from '@shared/runtime/objectModel';

/**
 * `resolved()` builds a fresh record on every call, so cache the last one per object. Keyed on
 * the version the object was at, which is exactly when the record can go stale.
 */
const resolvedCache = new WeakMap<RuntimeObject, { version: number; props: Record<string, PropValue> }>();

/** The object's properties at `version`, reusing the previous record when nothing changed. */
function resolvedAt(obj: RuntimeObject, version: number): Record<string, PropValue> {
  const cached = resolvedCache.get(obj);
  if (cached && cached.version === version) return cached.props;
  const props = obj.resolved();
  resolvedCache.set(obj, { version, props });
  return props;
}

/** Subscribes to one runtime object and returns its current properties. */
export function useRuntimeObject(obj: RuntimeObject): Record<string, PropValue> {
  const version = useSyncExternalStore(obj.subscribe, obj.getSnapshot, obj.getSnapshot);
  return useMemo(() => resolvedAt(obj, version), [obj, version]);
}

/** Subscribes to the desktop itself, which changes when forms are created or released. */
export function useDesktop(desktop: Desktop): number {
  return useSyncExternalStore(desktop.subscribe, desktop.getSnapshot, desktop.getSnapshot);
}
